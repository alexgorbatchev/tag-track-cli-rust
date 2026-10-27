use crate::{CancellationToken, Error, Result};
use clap::ArgMatches;
use cli_command_kit::Mode;
use std::{
    io::{IsTerminal, Write},
    sync::Arc,
};
pub fn dependency_manager() -> Result<cli_deps::Manager> {
    let bin = cli_deps::managed_bin_dir("tag-track")?;
    let runner = Arc::new(cli_deps::ProcessRunner {
        path: Some(cli_deps::managed_path(
            &bin,
            std::env::var_os("PATH").as_deref(),
        )?),
    });
    let installer = Arc::new(cli_deps::PackageInstaller {
        package: "ffmpeg".into(),
        runner: runner.clone(),
    });
    let mut dependencies = Vec::new();
    for name in ["ffmpeg", "ffprobe"] {
        let mut dependency = cli_deps::Dependency::new(name, "4.4");
        dependency.version_args = vec!["-version".into()];
        dependency.install_url = "https://ffmpeg.org/download.html".into();
        dependency.installer = Some(installer.clone());
        dependency.parse_version = Some(Arc::new(|_, banner| {
            cli_deps::default_version_parser("ffmpeg", &banner.replacen("ffprobe", "ffmpeg", 1))
        }));
        dependencies.push(dependency);
    }
    Ok(cli_deps::Manager::new(cli_deps::Config {
        app_name: "tag-track".into(),
        dependencies,
        runner: Some(runner),
        ..Default::default()
    })?)
}
pub async fn ensure(
    manager: &cli_deps::Manager,
    auto: bool,
    mode: Mode,
    token: &CancellationToken,
) -> Result<()> {
    let reports = manager.verify(token).await?;
    if reports.iter().all(|r| r.satisfied) {
        return Ok(());
    }
    let accepted = auto
        || mode == Mode::Human
            && std::io::stdin().is_terminal()
            && dialoguer::Confirm::new()
                .with_prompt("Install missing or outdated audio tools?")
                .default(false)
                .interact()
                .map_err(|e| Error::Invalid(e.to_string()))?;
    if accepted {
        manager.install_unsatisfied(token).await?.into_result()?;
        if manager
            .verify(token)
            .await?
            .iter()
            .all(|report| report.satisfied)
        {
            return Ok(());
        }
        return Err(Error::Invalid(
            "audio tools remain unavailable or outdated after installation".into(),
        ));
    }
    Err(Error::Invalid("required audio tools missing or outdated; run tag-track dependencies install or use --auto-install".into()))
}
pub fn release_channel(values: [Option<String>; 3]) -> Result<cli_deps::Updater> {
    let [base, owner, repo] = values;
    let channel=match (base,owner,repo) {
        (None,None,None)=>None,
        (Some(base),Some(owner),Some(repo))=>Some(cli_deps::ReleaseChannel::new(&base,owner,repo)?),
        _=>return Err(Error::Invalid("set TAG_TRACK_RUST_RELEASE_BASE_URL, TAG_TRACK_RUST_RELEASE_OWNER, and TAG_TRACK_RUST_RELEASE_REPO together".into())),
    };
    Ok(cli_deps::Updater { channel })
}
pub async fn dependencies(
    manager: &cli_deps::Manager,
    matches: &ArgMatches,
    token: &CancellationToken,
    output: &mut impl Write,
) -> Result<()> {
    match matches.subcommand() {
        None | Some(("check", _)) => {
            let reports = manager.verify(token).await?;
            for report in &reports {
                writeln!(
                    output,
                    "{}: {} (installed {}, required {})",
                    report.name,
                    if report.satisfied { "OK" } else { "FAIL" },
                    report.detected_version,
                    report.min_version
                )?;
            }
            if reports.iter().any(|r| !r.satisfied) {
                return Err(Error::Invalid(
                    "required dependencies are missing or outdated".into(),
                ));
            }
        }
        Some((verb, args)) => {
            let names = args
                .get_many::<String>("names")
                .map(|v| v.cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            if names.is_empty() {
                let outcome = if verb == "install" {
                    manager.install_unsatisfied(token).await?
                } else {
                    manager.update_all(token).await?
                };
                outcome.into_result()?;
            } else {
                for name in names {
                    if verb == "install" {
                        manager.install(&name, token).await?;
                    } else {
                        manager.update(&name, token).await?;
                    }
                    writeln!(output, "{name}: OK")?;
                }
            }
        }
    }
    Ok(())
}
