use crate::{
    CancellationToken, Error, Result, input,
    progress::{Event, MetadataInfo, ResultInfo},
};
use audio_metadata::{
    metadata::{Metadata, Resolver},
    tag::TagUpdate,
};
use cli_event_stream::Reporter;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub out_dir: Option<PathBuf>,
    pub in_place: bool,
    pub dry_run: bool,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub source_url: Option<String>,
    pub artwork: Option<String>,
    pub artwork_only: bool,
}
#[derive(Debug)]
pub struct Outcome {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub dry_run: bool,
}
pub struct Workflow {
    pub resolver: Resolver,
}

fn target_path(source: &Path, metadata: &Metadata, options: &Options) -> Result<PathBuf> {
    let directory = if options.in_place {
        None
    } else {
        options.out_dir.as_deref()
    };
    let directory = directory.unwrap_or_else(|| source.parent().unwrap_or(Path::new(".")));
    let extension = source
        .extension()
        .ok_or_else(|| Error::Invalid("audio file has no extension".into()))?;
    let mut filename = if metadata.artist.is_empty() {
        let stem = source.file_stem().unwrap_or_default();
        stem.to_str()
            .map(|name| {
                std::ffi::OsString::from(audio_metadata::tag::clean_filename(&input::search_name(
                    name,
                )))
            })
            .unwrap_or_else(|| stem.to_owned())
    } else {
        std::ffi::OsString::from(audio_metadata::tag::clean_filename(&input::search_name(
            &format!("{} - {}", metadata.artist, metadata.title),
        )))
    };
    filename.push(".");
    filename.push(extension);
    Ok(directory.join(filename))
}

impl Workflow {
    pub async fn process(
        &self,
        source: &Path,
        options: &Options,
        reporter: &Reporter,
        step: usize,
        total: usize,
        token: &CancellationToken,
    ) -> Result<Outcome> {
        audio_metadata::check_cancel(token)?;
        if !std::fs::metadata(source)?.is_file() {
            return Err(Error::Invalid(format!(
                "not an audio file: {}",
                source.display()
            )));
        }
        reporter
            .emit(Event::new("phase_start", "identify", step, total), token)
            .await?;
        let mut metadata = if options.artwork_only {
            let existing =
                audio_metadata::inspect::inspect(&self.resolver.tools, source, token).await?;
            Metadata {
                title: existing.title,
                artist: existing.artist,
                album: existing.album,
                genre: existing.genre,
                release_date: existing.date,
                provider: "Direct Artwork".into(),
                ..Default::default()
            }
        } else {
            let name = source.file_stem().and_then(|v| v.to_str()).ok_or_else(|| {
                Error::Invalid(
                    "track search requires a UTF-8 filename; use artwork embed for native paths"
                        .into(),
                )
            })?;
            let query = input::search_name(name);
            let (artist, title) = query.split_once(" - ").unwrap_or(("", &query));
            let artist = options.artist.as_deref().unwrap_or(artist);
            let title = options.title.as_deref().unwrap_or(title);
            self.resolver
                .resolve(source, &query, artist, title, token)
                .await?
        };
        for (target, override_value) in [
            (&mut metadata.artist, &options.artist),
            (&mut metadata.title, &options.title),
            (&mut metadata.album, &options.album),
            (&mut metadata.genre, &options.genre),
            (&mut metadata.release_date, &options.date),
            (&mut metadata.cover_url, &options.artwork),
            (&mut metadata.audio_source_url, &options.source_url),
        ] {
            if let Some(value) = override_value {
                *target = value.clone();
            }
        }
        metadata.fetched_at = chrono::Utc::now();
        let mut event = Event::new("metadata_found", "identify", step, total);
        event.metadata = Some(MetadataInfo::from(&metadata));
        reporter.emit(event, token).await?;
        let target = target_path(source, &metadata, options)?;
        if !options.dry_run {
            let artwork = if metadata.cover_url.is_empty() {
                None
            } else {
                let bytes = if metadata.cover_url.starts_with("http://")
                    || metadata.cover_url.starts_with("https://")
                {
                    self.resolver.artwork(&metadata.cover_url, token).await?
                } else {
                    let path = input::expand_home(Path::new(&metadata.cover_url));
                    let file = std::fs::File::open(path)?;
                    use std::io::Read;
                    let mut bytes = Vec::new();
                    file.take(20 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
                    if bytes.len() > 20 * 1024 * 1024 {
                        return Err(Error::Invalid("artwork exceeds 20 MiB".into()));
                    }
                    bytes
                };
                reporter
                    .emit(
                        Event::new("artwork_downloaded", "artwork", step, total),
                        token,
                    )
                    .await?;
                Some(bytes)
            };
            let update = if options.artwork_only {
                TagUpdate {
                    artwork,
                    ..Default::default()
                }
            } else {
                TagUpdate {
                    title: Some(metadata.title.clone()),
                    artist: (!metadata.artist.is_empty()).then(|| metadata.artist.clone()),
                    album: (!metadata.album.is_empty()).then(|| metadata.album.clone()),
                    genre: (!metadata.genre.is_empty()).then(|| metadata.genre.clone()),
                    date: (!metadata.release_date.is_empty())
                        .then(|| metadata.release_date.clone()),
                    comment: Some(options.comment.clone().unwrap_or_else(|| {
                        format!(
                            "Source: {} | Metadata: {} | Tagged: {}",
                            if metadata.audio_source_url.is_empty() {
                                "local file"
                            } else {
                                &metadata.audio_source_url
                            },
                            metadata.provider,
                            metadata.fetched_at.format("%Y-%m-%d")
                        )
                    })),
                    artwork,
                }
            };
            let input = source.to_owned();
            let output = target.clone();
            let cancellation = token.clone();
            tokio::task::spawn_blocking(move || {
                audio_metadata::tag::write(&input, &output, &update, &cancellation)
            })
            .await
            .map_err(|e| Error::Invalid(format!("audio tag worker: {e}")))??;
            if options.in_place && std::fs::canonicalize(source)? != std::fs::canonicalize(&target)?
            {
                std::fs::remove_file(source).map_err(|e| {
                    Error::Invalid(format!(
                        "tagged output retained at {}, but source removal failed: {e}",
                        target.display()
                    ))
                })?;
            }
            reporter
                .emit(Event::new("tagged", "tag", step, total), token)
                .await?;
        }
        let mut event = Event::new("complete", "complete", step, total);
        event.result = Some(ResultInfo {
            path: target.clone(),
            title: &metadata.title,
            artist: &metadata.artist,
            album: &metadata.album,
            release_year: metadata.release_year(),
            source: &metadata.provider,
            status: if options.dry_run { "dry_run" } else { "ok" },
        });
        reporter.emit(event, token).await?;
        Ok(Outcome {
            path: target,
            metadata,
            dry_run: options.dry_run,
        })
    }
}
