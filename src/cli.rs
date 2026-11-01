mod commands;
mod dependencies;
mod output;
use crate::{CancellationToken, Error, Options, Result, Workflow, input, progress::Event};
use audio_metadata::{
    cache::Cache,
    metadata::{Endpoints, Resolver},
    process::Tools,
};
use clap::ArgMatches;
use cli_command_kit::Mode;
use cli_event_stream::{Reporter, Target};
pub use commands::{command, help_options};
pub use dependencies::{dependencies, dependency_manager, ensure, release_channel};
pub use output::{print_inspection, print_outcome};
use std::{
    io::{IsTerminal, Write},
    path::PathBuf,
};
fn get_string(matches: &ArgMatches, name: &str) -> Option<String> {
    matches.try_get_one::<String>(name).ok().flatten().cloned()
}
fn get_flag(matches: &ArgMatches, name: &str) -> bool {
    matches
        .try_get_one::<bool>(name)
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}
pub fn options(matches: &ArgMatches, artwork_only: bool) -> Options {
    Options {
        out_dir: matches
            .try_get_one::<PathBuf>("out-dir")
            .ok()
            .flatten()
            .map(|p| input::expand_home(p)),
        in_place: get_flag(matches, "in-place"),
        dry_run: get_flag(matches, "dry-run"),
        artist: get_string(matches, "artist"),
        title: get_string(matches, "title"),
        album: get_string(matches, "album"),
        date: get_string(matches, "date"),
        genre: get_string(matches, "genre"),
        comment: get_string(matches, "comment"),
        source_url: get_string(matches, "source-url"),
        artwork: get_string(matches, "artwork").or_else(|| get_string(matches, "url")),
        artwork_only,
    }
}
pub async fn execute(matches: &ArgMatches, mode: Mode, token: &CancellationToken) -> Result<()> {
    let mut stdout = std::io::stdout();
    if matches.get_flag("version") {
        writeln!(stdout, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(());
    }
    if cli_command_kit::write_skill(&command(mode)?, matches, &help_options(), &mut stdout)? {
        return Ok(());
    }
    if matches.subcommand_name() == Some("self") {
        let updater = release_channel(
            [
                "TAG_TRACK_RUST_RELEASE_BASE_URL",
                "TAG_TRACK_RUST_RELEASE_OWNER",
                "TAG_TRACK_RUST_RELEASE_REPO",
            ]
            .map(|name| std::env::var(name).ok()),
        )?;
        let update = updater
            .update_running(env!("CARGO_PKG_VERSION"), token)
            .await?;
        let (state, version) = match update {
            cli_deps::Update::Current(v) => ("current", v),
            cli_deps::Update::Installed(v) => ("installed", v),
        };
        writeln!(stdout, "update: {state}\nversion: {version}")?;
        return Ok(());
    }
    let manager = dependency_manager()?;
    if let Some(("dependencies", sub)) = matches.subcommand() {
        return dependencies(&manager, sub, token, &mut stdout).await;
    }
    let (operation, selected) = match matches.subcommand() {
        Some(("track", sub)) => sub.subcommand().expect("required track verb"),
        Some(("artwork", sub)) => sub.subcommand().expect("required artwork verb"),
        _ => ("update", matches),
    };
    let mut paths = selected
        .get_many::<PathBuf>("files")
        .map(|files| files.map(|p| input::expand_home(p)).collect::<Vec<_>>())
        .unwrap_or_default();
    if paths.is_empty() && !std::io::stdin().is_terminal() || paths == [PathBuf::from("-")] {
        let raw = input::read_stdin(token).await?;
        paths = input::files_from_lines(&raw)?;
    }
    if paths.is_empty() {
        command(mode)?.print_help()?;
        return Ok(());
    }
    ensure(&manager, matches.get_flag("auto-install"), mode, token).await?;
    let tools = Tools::new(
        cli_deps::managed_path(manager.bin_dir(), std::env::var_os("PATH").as_deref())?,
        std::env::current_dir()?,
    );
    let mut errors = Vec::new();
    if operation == "inspect" {
        for path in paths {
            match audio_metadata::inspect::inspect(&tools, &path, token).await {
                Ok(info) => print_inspection(&info, mode, &mut stdout)?,
                Err(e) => {
                    let e = Error::from(e);
                    if crate::exit_code(&e) == 130 {
                        return Err(e);
                    }
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
    } else {
        let cache_root = dirs::cache_dir()
            .ok_or_else(|| Error::Invalid("cache directory unavailable".into()))?
            .join("tag-track");
        let cache = Cache::new(
            cache_root,
            !get_flag(selected, "no-cache") && !get_flag(selected, "refresh"),
        );
        let flow = Workflow {
            resolver: Resolver::new(
                tools,
                cache,
                Endpoints::default(),
                concat!("tag-track/", env!("CARGO_PKG_VERSION")),
            )?,
        };
        let target = get_string(selected, "progress-target")
            .or_else(|| get_string(selected, "progress-socket").map(|p| format!("unix://{p}")))
            .or_else(|| {
                std::env::var("TAG_TRACK_PROGRESS_TARGET")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            });
        let target = target.as_deref().map(Target::parse).transpose()?;
        let opts = options(selected, operation == "embed");
        if target.is_some()
            && (paths.iter().any(|path| path.to_str().is_none())
                || opts
                    .out_dir
                    .as_ref()
                    .is_some_and(|path| path.to_str().is_none()))
        {
            return Err(Error::Invalid(
                "NDJSON progress requires UTF-8 input/output paths".into(),
            ));
        }
        let reporter = match &target {
            Some(target) => Reporter::connect(target, token).await?,
            None => Reporter::default(),
        };
        let silent = matches!(target, Some(Target::Stdout | Target::Stderr));
        let spinner = if mode == Mode::Human
            && std::io::stderr().is_terminal()
            && !matches.get_flag("verbose")
            && !silent
        {
            let p = indicatif::ProgressBar::new_spinner();
            p.set_message("Identifying and tagging audio");
            p.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(p)
        } else {
            None
        };
        for (index, path) in paths.iter().enumerate() {
            if matches.get_flag("verbose") && !silent {
                writeln!(
                    std::io::stderr(),
                    "Processing: {}",
                    cli_command_kit::display_path(path)
                )?;
            }
            match flow
                .process(path, &opts, &reporter, index + 1, paths.len(), token)
                .await
            {
                Ok(outcome) => {
                    if matches.get_flag("verbose") && !silent {
                        writeln!(
                            std::io::stderr(),
                            "Metadata source: {}",
                            cli_command_kit::display_message(&outcome.metadata.provider)
                        )?;
                    }
                    if !silent {
                        print_outcome(&outcome, mode, &mut stdout)?;
                    }
                }
                Err(e) => {
                    if crate::exit_code(&e) == 130 {
                        return Err(e);
                    }
                    if matches!(e, Error::Progress(_)) {
                        return Err(e);
                    }
                    let message =
                        cli_command_kit::display_message(&format!("{}: {e}", path.display()));
                    let mut event = Event::new("error", "error", index + 1, paths.len());
                    event.error = &message;
                    reporter.emit(event, token).await?;
                    errors.push(message);
                }
            }
        }
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Batch(errors.join("; ")))
    }
}
