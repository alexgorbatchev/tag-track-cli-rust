use crate::{Error, Result};
use clap::{Arg, ArgAction, Command};
use cli_command_kit::{HelpOptions, Mode};
use cli_event_stream::Target;
use std::path::PathBuf;
pub const SKILL: &str = "Install the tag-track skill using the Skills CLI:\n\n  npx skills add alexgorbatchev/tag-track-cli --skill tag-track\n\nSkill:\n\n  https://github.com/alexgorbatchev/tag-track-cli/tree/main/skills/tag-track\n";
pub fn help_options() -> HelpOptions {
    HelpOptions {
        skill: Some(SKILL.into()),
        ..Default::default()
    }
}
fn flag(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}
fn files() -> Arg {
    Arg::new("files")
        .value_name("FILE")
        .num_args(0..)
        .value_parser(clap::value_parser!(PathBuf))
}
fn update_options(command: Command) -> Command {
    let mut command = command
        .arg(files())
        .arg(
            Arg::new("out-dir")
                .long("out-dir")
                .short('o')
                .value_name("DIRECTORY")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Save tagged copies here; default: each source directory"),
        )
        .arg(
            flag(
                "in-place",
                "Replace the source after the tagged output is committed",
            )
            .short('i'),
        )
        .arg(flag(
            "refresh",
            "Bypass caches and identify the audio again",
        ))
        .arg(flag("no-cache", "Bypass metadata and artwork caches"))
        .arg(flag(
            "dry-run",
            "Preview metadata without modifying audio files",
        ));
    for (name, help) in [
        ("artist", "Override artist"),
        ("title", "Override track title"),
        ("album", "Override album"),
        ("date", "Override release date: YYYY or YYYY-MM-DD"),
        ("genre", "Override genre"),
        ("comment", "Override provenance comment"),
        ("source-url", "Record this source in provenance"),
        ("artwork", "Embed artwork from an image path or HTTP(S) URL"),
    ] {
        let mut arg = Arg::new(name).long(name).value_name("VALUE").help(help);
        if name == "date" {
            arg = arg.value_parser(parse_date);
        }
        command = command.arg(arg);
    }
    command
        .arg(
            Arg::new("progress-target")
                .long("progress-target")
                .value_name("URI")
                .value_parser(parse_target)
                .help("Stream NDJSON to stdout, stderr, fd://, unix://, or tcp://"),
        )
        .arg(
            Arg::new("progress-socket")
                .long("progress-socket")
                .value_name("PATH")
                .help("Stream NDJSON to a Unix socket"),
        )
}
pub fn command(mode: Mode) -> Result<Command> {
    let root = update_options(
        Command::new("tag-track").about("Inspect, identify, and tag audio without transcoding"),
    )
    .disable_version_flag(true)
    .arg(flag("version", "Print the raw version").global(true))
    .arg(
        flag("verbose", "Show detailed progress")
            .short('v')
            .global(true),
    )
    .arg(flag("auto-install", "Install missing tools without prompting").global(true))
    .subcommand(
        Command::new("track")
            .about("Inspect and update audio tracks")
            .subcommand_required(true)
            .subcommand(update_options(
                Command::new("update").about("Identify and update tags and cover art"),
            ))
            .subcommand(
                Command::new("inspect")
                    .about("Read existing audio tags and cover dimensions")
                    .arg(files()),
            ),
    )
    .subcommand(
        Command::new("artwork")
            .about("Manage cover art")
            .subcommand_required(true)
            .subcommand(
                Command::new("embed")
                    .about("Embed square cover art while preserving existing tags")
                    .arg(files())
                    .arg(
                        Arg::new("artwork")
                            .long("artwork")
                            .value_name("IMAGE")
                            .required_unless_present("url")
                            .conflicts_with("url")
                            .help("Image path or HTTP(S) URL"),
                    )
                    .arg(
                        Arg::new("url")
                            .long("url")
                            .value_name("URL")
                            .value_parser(parse_url)
                            .help("Download this HTTP(S) artwork image"),
                    )
                    .arg(
                        Arg::new("out-dir")
                            .long("out-dir")
                            .short('o')
                            .value_name("DIRECTORY")
                            .value_parser(clap::value_parser!(PathBuf))
                            .help("Save copies in this directory"),
                    )
                    .arg(
                        flag("in-place", "Replace the source after successful tagging").short('i'),
                    ),
            ),
    )
    .subcommand(
        Command::new("dependencies")
            .about("Manage required audio tools")
            .subcommand(Command::new("check").about("Verify ffmpeg and ffprobe versions"))
            .subcommand(dependency_action(
                "install",
                "Install named or missing tools",
            ))
            .subcommand(dependency_action("update", "Update named or all tools")),
    )
    .subcommand(
        Command::new("self")
            .about("Manage this executable")
            .subcommand_required(true)
            .subcommand(Command::new("update").about("Install a published Rust release")),
    );
    cli_command_kit::setup(root, mode, &help_options()).map_err(|e| Error::Invalid(e.to_string()))
}
fn dependency_action(name: &'static str, help: &'static str) -> Command {
    Command::new(name).about(help).arg(
        Arg::new("names")
            .value_name("DEPENDENCY")
            .num_args(0..)
            .value_parser(["ffmpeg", "ffprobe"]),
    )
}
fn parse_date(raw: &str) -> std::result::Result<String, String> {
    if raw.len() == 4 && raw.bytes().all(|b| b.is_ascii_digit())
        || chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_ok() && raw.len() == 10
    {
        Ok(raw.into())
    } else {
        Err("date must be YYYY or a valid YYYY-MM-DD".into())
    }
}
fn parse_url(raw: &str) -> std::result::Result<String, String> {
    let url = url::Url::parse(raw).map_err(|e| e.to_string())?;
    if matches!(url.scheme(), "http" | "https") && url.host().is_some() {
        Ok(raw.into())
    } else {
        Err("artwork URL must use HTTP(S)".into())
    }
}
fn parse_target(raw: &str) -> std::result::Result<String, String> {
    Target::parse(raw)
        .map(|_| raw.into())
        .map_err(|e| e.to_string())
}
