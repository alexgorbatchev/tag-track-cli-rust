use audio_metadata::{
    cache::Cache,
    metadata::{Endpoints, Metadata, Resolver},
    process::Tools,
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tag_track_cli::{
    CancellationToken, Options, Workflow,
    input::{files_from_lines, search_name},
};

fn temp() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
fn tools() -> Tools {
    Tools::new(
        std::env::var_os("PATH").unwrap(),
        std::env::current_dir().unwrap(),
    )
}
fn audio(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    assert!(
        std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:a",
                "flac"
            ])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    path
}
fn workflow(path: &Path, cache: Cache) -> Workflow {
    let query = search_name(path.file_stem().unwrap().to_str().unwrap());
    let (artist, title) = query.split_once(" - ").unwrap_or(("", &query));
    let key = format!(
        "{}:{query}:{artist}:{title}",
        Cache::file_key(path).unwrap()
    );
    cache
        .put(
            "metadata",
            &key,
            &Metadata {
                artist: "Artist".into(),
                title: "Song".into(),
                album: "Album".into(),
                release_date: "2020-04-03".into(),
                provider: "Fixture cache".into(),
                ..Default::default()
            },
            Duration::from_secs(60),
        )
        .unwrap();
    Workflow {
        resolver: Resolver::new(tools(), cache, Endpoints::default(), "tag-track-test").unwrap(),
    }
}
#[test]
fn input_pipeline_prefers_results_and_sanitizes_search_identity() {
    assert_eq!(
        files_from_lines("working\nDONE: /music/a.flac\noutput: /music/b.mp3\n").unwrap(),
        vec![
            PathBuf::from("/music/a.flac"),
            PathBuf::from("/music/b.mp3")
        ]
    );
    assert_eq!(
        files_from_lines("/music/a.flac\n\n/music/b.mp3")
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        search_name("Beyoncé — Song [dQw4w9WgXcQ] [Remix]"),
        "Beyoncé - Song [Remix]"
    );
}
#[tokio::test]
async fn real_tags_honor_comment_date_and_preserve_source_audio() {
    let tmp = temp();
    let source = audio(tmp.path(), "Artist - Song.flac");
    let before = std::fs::read(&source).unwrap();
    let flow = workflow(&source, Cache::new(tmp.path().join("cache"), true));
    let opts = Options {
        out_dir: Some(tmp.path().join("tagged")),
        comment: Some("Explicit comment".into()),
        ..Default::default()
    };
    let result = flow
        .process(
            &source,
            &opts,
            &Default::default(),
            1,
            1,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let info = audio_metadata::inspect::inspect(&tools(), &result.path, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(info.title, "Song");
    assert_eq!(info.date, "2020-04-03");
    assert_eq!(info.comment, "Explicit comment");
    assert_eq!(std::fs::read(source).unwrap(), before);
}
#[tokio::test]
async fn dry_run_and_cancel_do_not_modify_files() {
    let tmp = temp();
    let source = audio(tmp.path(), "Artist - Song.flac");
    let before = std::fs::read(&source).unwrap();
    let flow = workflow(&source, Cache::new(tmp.path().join("cache"), true));
    let opts = Options {
        dry_run: true,
        out_dir: Some(tmp.path().join("tagged")),
        ..Default::default()
    };
    let outcome = flow
        .process(
            &source,
            &opts,
            &Default::default(),
            1,
            1,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(outcome.dry_run);
    assert!(!tmp.path().join("tagged").exists());
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        flow.process(
            &source,
            &Options::default(),
            &Default::default(),
            1,
            1,
            &token
        )
        .await,
        Err(tag_track_cli::Error::Audio(
            audio_metadata::Error::Cancelled
        ))
    ));
    assert_eq!(std::fs::read(source).unwrap(), before);
}

#[tokio::test]
async fn artwork_preserves_tags_and_embeds_square_image_with_native_filename() {
    use std::os::unix::ffi::OsStringExt;
    let tmp = temp();
    let source = audio(tmp.path(), "original.flac");
    audio_metadata::tag::write(
        &source,
        &source,
        &audio_metadata::tag::TagUpdate {
            title: Some("Existing".into()),
            album: Some("Keep album".into()),
            comment: Some("Keep comment".into()),
            ..Default::default()
        },
        &CancellationToken::new(),
    )
    .unwrap();
    let native = tmp
        .path()
        .join(std::ffi::OsString::from_vec(b"native-\xff.flac".to_vec()));
    std::fs::rename(source, &native).unwrap();
    let image = tmp.path().join("cover.png");
    image::RgbImage::from_pixel(40, 20, image::Rgb([20, 40, 60]))
        .save(&image)
        .unwrap();
    let flow = Workflow {
        resolver: Resolver::new(
            tools(),
            Cache::new(tmp.path().join("cache"), true),
            Endpoints::default(),
            "test",
        )
        .unwrap(),
    };
    let result = flow
        .process(
            &native,
            &Options {
                artwork_only: true,
                artwork: Some(image.to_str().unwrap().into()),
                out_dir: Some(tmp.path().join("out")),
                ..Default::default()
            },
            &Default::default(),
            1,
            1,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(result.path.file_name(), native.file_name());
    let info = audio_metadata::inspect::inspect(&tools(), &result.path, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(info.title, "Existing");
    assert_eq!(info.album, "Keep album");
    assert_eq!(info.comment, "Keep comment");
    assert_eq!((info.artwork_width, info.artwork_height), (1400, 1400));
    assert!(native.exists());
}

#[tokio::test]
async fn in_place_commit_renames_and_collision_retains_both_files() {
    let tmp = temp();
    let source = audio(tmp.path(), "Artist - Song [Remix].flac");
    let flow = workflow(&source, Cache::new(tmp.path().join("cache"), true));
    let token = CancellationToken::new();
    let destination = tmp.path().join("Artist - Song.flac");
    std::fs::write(&destination, b"retain existing").unwrap();
    let before = std::fs::read(&source).unwrap();
    assert!(
        flow.process(
            &source,
            &Options {
                in_place: true,
                ..Default::default()
            },
            &Default::default(),
            1,
            1,
            &token
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(&destination).unwrap(), b"retain existing");
    assert_eq!(std::fs::read(&source).unwrap(), before);
    std::fs::remove_file(&destination).unwrap();
    let result = flow
        .process(
            &source,
            &Options {
                in_place: true,
                out_dir: Some(tmp.path().join("ignored")),
                source_url: Some("https://example.com/track".into()),
                ..Default::default()
            },
            &Default::default(),
            1,
            1,
            &token,
        )
        .await
        .unwrap();
    assert_eq!(result.path, destination);
    assert!(!source.exists());
    assert!(!tmp.path().join("ignored").exists());
    let info = audio_metadata::inspect::inspect(&tools(), &destination, &token)
        .await
        .unwrap();
    assert!(
        info.comment
            .contains("Source: https://example.com/track | Metadata: Fixture cache | Tagged:")
    );
}

#[tokio::test]
async fn artwork_download_is_real_local_http_and_errors_leave_source_unchanged() {
    let tmp = temp();
    let source = audio(tmp.path(), "plain.flac");
    let before = std::fs::read(&source).unwrap();
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 30))
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let bytes = image.into_inner();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route(
        "/cover",
        axum::routing::get(move || {
            let b = bytes.clone();
            async move { b }
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let flow = Workflow {
        resolver: Resolver::new(
            tools(),
            Cache::new(tmp.path().join("cache"), true),
            Endpoints::default(),
            "test",
        )
        .unwrap(),
    };
    let opts = Options {
        artwork_only: true,
        artwork: Some(format!("http://{address}/cover")),
        out_dir: Some(tmp.path().join("tagged")),
        ..Default::default()
    };
    let result = flow
        .process(
            &source,
            &opts,
            &Default::default(),
            1,
            1,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(
        audio_metadata::inspect::inspect(&tools(), &result.path, &CancellationToken::new())
            .await
            .unwrap()
            .has_artwork
    );
    let bad = Options {
        artwork: Some(tmp.path().join("missing.png").to_str().unwrap().into()),
        out_dir: Some(tmp.path().join("missing")),
        ..opts
    };
    assert!(
        flow.process(
            &source,
            &bad,
            &Default::default(),
            1,
            1,
            &CancellationToken::new()
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert!(!tmp.path().join("missing").exists());
    server.abort();
    assert!(
        flow.process(
            tmp.path(),
            &Options::default(),
            &Default::default(),
            1,
            1,
            &CancellationToken::new()
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("not an audio file")
    );
}

#[tokio::test]
async fn progress_is_owned_ordered_ndjson_and_dry_run_skips_missing_artwork() {
    use tokio::io::AsyncReadExt;
    let tmp = temp();
    let source = audio(tmp.path(), "Artist - Song.flac");
    let flow = workflow(&source, Cache::new(tmp.path().join("cache"), true));
    let (read, write) = tokio::io::duplex(16384);
    let reporter = cli_event_stream::Reporter::writer(write);
    let result = flow
        .process(
            &source,
            &Options {
                dry_run: true,
                artwork: Some("missing.jpg".into()),
                ..Default::default()
            },
            &reporter,
            2,
            3,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    drop(reporter);
    let mut read = read;
    let mut raw = String::new();
    read.read_to_string(&mut raw).await.unwrap();
    let events: Vec<serde_json::Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        events
            .iter()
            .map(|e| e["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["phase_start", "metadata_found", "complete"]
    );
    assert_eq!(events[2]["result"]["status"], "dry_run");
    assert_eq!(events[2]["result"]["path"], result.path.to_str().unwrap());
    assert_eq!(events[2]["step"], 2);
    assert_eq!(events[2]["total_steps"], 3);
    assert_eq!(events[1]["metadata"]["release_date"], "2020-04-03");
}

#[tokio::test]
async fn metadata_overrides_survive_tags_and_artwork_failures_are_atomic() {
    let tmp = temp();
    let source = audio(tmp.path(), "Artist - Song.flac");
    let cache = Cache::new(tmp.path().join("cache"), true);
    let key = format!(
        "{}:Artist - Song:Override artist:Override title",
        Cache::file_key(&source).unwrap()
    );
    cache
        .put(
            "metadata",
            &key,
            &Metadata {
                title: "Original title".into(),
                artist: "Original artist".into(),
                ..Default::default()
            },
            Duration::from_secs(60),
        )
        .unwrap();
    let flow = Workflow {
        resolver: Resolver::new(tools(), cache, Endpoints::default(), "test").unwrap(),
    };
    let opts = Options {
        artist: Some("Override artist".into()),
        title: Some("Override title".into()),
        album: Some("Override album".into()),
        genre: Some("Jazz".into()),
        date: Some("2024".into()),
        out_dir: Some(tmp.path().join("out")),
        ..Default::default()
    };
    let result = flow
        .process(
            &source,
            &opts,
            &Default::default(),
            1,
            1,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let info = audio_metadata::inspect::inspect(&tools(), &result.path, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(info.artist, "Override artist");
    assert_eq!(info.title, "Override title");
    assert_eq!(info.album, "Override album");
    assert_eq!(info.genre, "Jazz");
    assert_eq!(info.date, "2024");
    assert!(info.comment.starts_with("Source: local file | Metadata:"));
    let huge = tmp.path().join("huge.jpg");
    std::fs::File::create(&huge)
        .unwrap()
        .set_len(20 * 1024 * 1024 + 1)
        .unwrap();
    let opts = Options {
        artwork_only: true,
        artwork: Some(huge.to_str().unwrap().into()),
        out_dir: Some(tmp.path().join("bad")),
        ..Default::default()
    };
    assert!(
        flow.process(
            &source,
            &opts,
            &Default::default(),
            1,
            1,
            &CancellationToken::new()
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("20 MiB")
    );
    assert!(!tmp.path().join("bad").exists());
}
