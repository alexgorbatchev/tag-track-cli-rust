use cli_command_kit::Mode;
use std::{io::Write, path::Path, process::Command, sync::Arc};
use tag_track_cli::{CancellationToken, cli};
fn temp() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
fn binary() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_tag-track"));
    c.env("AGENT", "0")
        .env(
            "XDG_DATA_HOME",
            concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp/data"),
        )
        .env_remove("TAG_TRACK_RUST_RELEASE_BASE_URL")
        .env_remove("TAG_TRACK_RUST_RELEASE_OWNER")
        .env_remove("TAG_TRACK_RUST_RELEASE_REPO")
        .env_remove("TAG_TRACK_PROGRESS_TARGET");
    c
}
struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken output"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[test]
fn native_help_version_skill_and_validation() {
    for mode in [Mode::Human, Mode::Agent] {
        for path in [
            vec![],
            vec!["track"],
            vec!["track", "update"],
            vec!["track", "inspect"],
            vec!["artwork"],
            vec!["artwork", "embed"],
            vec!["dependencies"],
            vec!["dependencies", "check"],
            vec!["dependencies", "install"],
            vec!["dependencies", "update"],
            vec!["self"],
            vec!["self", "update"],
            vec!["skill"],
        ] {
            let mut args = vec!["tag-track"];
            args.extend(path);
            args.push("--help");
            let error = cli::command(mode)
                .unwrap()
                .try_get_matches_from(args)
                .unwrap_err();
            assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
            assert_eq!(error.to_string().starts_with("ALERT:"), mode == Mode::Agent);
        }
        let options = cli::help_options();
        let matches = cli::command(mode)
            .unwrap()
            .try_get_matches_from(["tag-track", "skill"])
            .unwrap();
        let mut bytes = Vec::new();
        assert!(cli_command_kit::write_skill(&matches, &options, &mut bytes).unwrap());
        assert_eq!(bytes, cli::SKILL.as_bytes());
        assert!(cli_command_kit::write_skill(&matches, &options, &mut Broken).is_err());
    }
    for args in [
        vec!["skill", "extra"],
        vec!["--date", "2026-02-30"],
        vec!["--date", "January"],
        vec!["--progress-target", "http://bad"],
        vec!["artwork", "embed"],
        vec!["artwork", "embed", "--url", "ftp://example.com/image"],
        vec!["dependencies", "install", "unknown"],
    ] {
        let mut all = vec!["tag-track"];
        all.extend(args);
        assert!(
            cli::command(Mode::Agent)
                .unwrap()
                .try_get_matches_from(all)
                .is_err()
        );
    }
    for date in ["2020", "2020-02-29"] {
        let matches = cli::command(Mode::Agent)
            .unwrap()
            .try_get_matches_from([
                "tag-track",
                "--date",
                date,
                "--in-place",
                "--dry-run",
                "--genre",
                "Jazz",
                "--album",
                "Album",
                "--title",
                "Song",
                "--artist",
                "Artist",
                "--source-url",
                "https://example.com",
            ])
            .unwrap();
        let o = cli::options(&matches, false);
        assert!(o.in_place && o.dry_run);
        assert_eq!(o.date.as_deref(), Some(date));
        assert_eq!(o.genre.as_deref(), Some("Jazz"));
        assert_eq!(o.source_url.as_deref(), Some("https://example.com"));
    }
    let version = binary().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(version.stdout, b"0.1.0\n");
    let empty = binary().env("AGENT", "1").output().unwrap();
    assert!(
        empty.status.success(),
        "{}",
        String::from_utf8_lossy(&empty.stderr)
    );
    assert!(empty.stdout.starts_with(b"ALERT:"));
    let skill = binary().args(["skill"]).output().unwrap();
    assert!(skill.status.success());
    assert_eq!(skill.stdout, cli::SKILL.as_bytes());
    let help = binary()
        .env("AGENT", "1")
        .args(["help", "track", "inspect"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(help.stdout.starts_with(b"ALERT:"));
}
#[tokio::test]
async fn unpublished_release_and_explicit_channel_are_typed() {
    let token = CancellationToken::new();
    let updater = cli::release_channel([None, None, None]).unwrap();
    assert!(matches!(
        updater
            .update_to("0.1.0", Path::new("unused"), &token)
            .await,
        Err(cli_deps::Error::Unpublished)
    ));
    assert!(cli::release_channel([Some("http://127.0.0.1".into()), None, None]).is_err());
    assert!(
        cli::release_channel([
            Some("http://example.com".into()),
            Some("owner".into()),
            Some("repo".into())
        ])
        .is_err()
    );
    assert!(
        cli::release_channel([
            Some("http://127.0.0.1".into()),
            Some("owner".into()),
            Some("repo".into())
        ])
        .unwrap()
        .channel
        .is_some()
    );
    let output = binary().args(["self", "update"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .matches("ERR:")
            .count(),
        1
    );
    assert!(output.stdout.is_empty());
}
fn install_script(path: &Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path.join(name), "#!/bin/sh\nprintf '9.0.0\\n'\n").unwrap();
    std::fs::set_permissions(path.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
}
fn manager(path: &Path, install: bool) -> cli_deps::Manager {
    let mut dependency = cli_deps::Dependency::new("ffmpeg", "4.4");
    dependency.installer = Some(Arc::new(cli_deps::InstallerFn(
        move |path: std::path::PathBuf, _: CancellationToken| async move {
            if install {
                install_script(&path, "ffmpeg");
            }
            Ok(())
        },
    )));
    cli_deps::Manager::new(cli_deps::Config {
        dependencies: vec![dependency],
        bin_dir: Some(path.to_owned()),
        runner: Some(Arc::new(cli_deps::ProcessRunner {
            path: Some(path.as_os_str().to_owned()),
        })),
        ..Default::default()
    })
    .unwrap()
}
#[tokio::test]
async fn dependencies_install_recheck_and_keep_noninteractive_consent_explicit() {
    let tmp = temp();
    let m = manager(tmp.path(), true);
    let token = CancellationToken::new();
    assert!(cli::ensure(&m, false, Mode::Agent, &token).await.is_err());
    cli::ensure(&m, true, Mode::Agent, &token).await.unwrap();
    cli::ensure(&m, false, Mode::Human, &token).await.unwrap();
    for args in [
        vec!["dependencies"],
        vec!["dependencies", "check"],
        vec!["dependencies", "install"],
        vec!["dependencies", "install", "ffmpeg"],
        vec!["dependencies", "update"],
        vec!["dependencies", "update", "ffmpeg"],
    ] {
        let mut all = vec!["tag-track"];
        all.extend(args);
        let matches = cli::command(Mode::Agent)
            .unwrap()
            .try_get_matches_from(all)
            .unwrap();
        let mut output = Vec::new();
        cli::dependencies(&m, matches.subcommand().unwrap().1, &token, &mut output)
            .await
            .unwrap();
    }
    let matches = cli::command(Mode::Agent)
        .unwrap()
        .try_get_matches_from(["tag-track", "dependencies", "check"])
        .unwrap();
    assert!(
        cli::dependencies(&m, matches.subcommand().unwrap().1, &token, &mut Broken)
            .await
            .is_err()
    );
    let bad = temp();
    let m = manager(bad.path(), false);
    assert!(
        cli::ensure(&m, true, Mode::Agent, &token).await.is_err(),
        "installer success cannot prove the tool is available"
    );
    let mut output = Vec::new();
    assert!(
        cli::dependencies(&m, matches.subcommand().unwrap().1, &token, &mut output)
            .await
            .is_err()
    );
}
#[test]
fn rendering_handles_modes_and_write_errors() {
    let metadata = audio_metadata::metadata::Metadata {
        artist: "Artist".into(),
        title: format!(
            "Song from [{}]",
            dirs::home_dir().unwrap().join("Music").display()
        ),
        album: "Album".into(),
        release_date: "2021-03-04".into(),
        cover_url: "https://example.com/cover.jpg".into(),
        provider: "Fixture".into(),
        ..Default::default()
    };
    for mode in [Mode::Human, Mode::Agent] {
        for dry_run in [false, true] {
            let outcome = tag_track_cli::Outcome {
                path: dirs::home_dir().unwrap().join("Music/song.flac"),
                metadata: metadata.clone(),
                dry_run,
            };
            let mut bytes = Vec::new();
            cli::print_outcome(&outcome, mode, &mut bytes).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("~/Music/song.flac"));
            assert!(text.contains("Song from [~/Music]"));
            if dry_run || mode == Mode::Agent {
                assert!(text.contains("Album"));
                assert!(text.contains("2021-03-04"));
                assert!(text.contains("https://example.com/cover.jpg"));
                assert!(text.contains("Fixture"));
            }
            assert!(cli::print_outcome(&outcome, mode, &mut Broken).is_err());
        }
    }
    let info = audio_metadata::inspect::Inspection::default();
    assert!(cli::print_inspection(&info, Mode::Human, &mut Broken).is_err());
}

#[test]
fn native_cli_pipeline_progress_inspection_artwork_and_partial_failure() {
    let tmp = temp();
    let source = tmp.path().join("Artist - Song.flac");
    assert!(
        Command::new("ffmpeg")
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
            .arg(&source)
            .status()
            .unwrap()
            .success()
    );
    let cache = audio_metadata::cache::Cache::new(tmp.path().join("cache/tag-track"), true);
    let key = format!(
        "{}:Artist - Song:Artist:Song",
        audio_metadata::cache::Cache::file_key(&source).unwrap()
    );
    cache
        .put(
            "metadata",
            &key,
            &audio_metadata::metadata::Metadata {
                title: "Song".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                release_date: "2021-03-04".into(),
                provider: "Fixture cache".into(),
                ..Default::default()
            },
            std::time::Duration::from_secs(60),
        )
        .unwrap();
    let tagged = tmp.path().join("tagged");
    let verbose = binary()
        .env("XDG_CACHE_HOME", tmp.path().join("cache"))
        .args(["--dry-run", "--verbose"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(verbose.status.success());
    let diagnostics = String::from_utf8(verbose.stderr).unwrap();
    assert!(diagnostics.contains("Processing:"));
    assert!(diagnostics.contains("Metadata source: Fixture cache"));
    let output = binary()
        .env("AGENT", "1")
        .env("XDG_CACHE_HOME", tmp.path().join("cache"))
        .arg(&source)
        .arg("--out-dir")
        .arg(&tagged)
        .args(["--comment", "Chosen comment"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("output:"));
    for mode in ["0", "1"] {
        let result = binary()
            .env("AGENT", mode)
            .args(["track", "inspect"])
            .arg(tagged.join("Artist - Song.flac"))
            .output()
            .unwrap();
        assert!(result.status.success());
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains("comment: Chosen comment"));
        assert!(text.contains("date: 2021-03-04"));
    }
    let preview = binary()
        .env("AGENT", "1")
        .env("XDG_CACHE_HOME", tmp.path().join("cache"))
        .env("TAG_TRACK_PROGRESS_TARGET", "stdout")
        .args(["track", "update", "--dry-run"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let events: Vec<serde_json::Value> = String::from_utf8(preview.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.last().unwrap()["result"]["status"], "dry_run");
    let mut child = binary()
        .env("XDG_CACHE_HOME", tmp.path().join("cache"))
        .args(["--dry-run", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "working\nDONE: {}",
        source.display()
    )
    .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("[DRY RUN]"));
    let partial = binary()
        .args(["track", "inspect"])
        .arg(&source)
        .arg(tmp.path().join("missing.flac"))
        .output()
        .unwrap();
    assert_eq!(partial.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&partial.stdout).contains("has_artwork: false"));
    assert_eq!(
        String::from_utf8_lossy(&partial.stderr)
            .matches("ERR:")
            .count(),
        1
    );
    let cover = tmp.path().join("cover.png");
    image::RgbImage::new(20, 10).save(&cover).unwrap();
    let embed = binary()
        .args(["artwork", "embed"])
        .arg(&source)
        .arg("--artwork")
        .arg(&cover)
        .arg("--out-dir")
        .arg(tmp.path().join("art"))
        .output()
        .unwrap();
    assert!(
        embed.status.success(),
        "{}",
        String::from_utf8_lossy(&embed.stderr)
    );
    let inspection = binary()
        .args(["track", "inspect"])
        .arg(tmp.path().join("art/Artist - Song.flac"))
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&inspection.stdout).contains("artwork_dimensions: 1400x1400"));
    let error = binary()
        .env("TAG_TRACK_PROGRESS_TARGET", "invalid")
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(error.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&error.stderr)
            .matches("ERR:")
            .count(),
        1
    );
    let batch = binary()
        .env("TAG_TRACK_PROGRESS_TARGET", "stdout")
        .arg(tmp.path().join("missing.flac"))
        .output()
        .unwrap();
    assert_eq!(batch.status.code(), Some(1));
    let event: serde_json::Value = serde_json::from_slice(&batch.stdout).unwrap();
    assert_eq!(event["type"], "error");
}

#[test]
fn pending_stdin_signal_cancels_process_without_runtime_hang() {
    let mut child = binary()
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(130));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "cancellation must finish runtime teardown"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn usage_guide_covers_live_commands_and_skill_location() {
    let text = [
        include_str!("../skills/tag-track/SKILL.md"),
        include_str!("../skills/tag-track/references/commands.md"),
        include_str!("../skills/tag-track/references/help.md"),
        include_str!("../skills/tag-track/references/operations.md"),
    ]
    .join("\n");
    let missing = cli_command_kit::skill_omissions(
        &cli::command(Mode::Agent).unwrap(),
        &cli::help_options(),
        &text,
    );
    assert!(missing.is_empty(), "{}", missing.join("\n"));
    assert!(include_str!("../skills/tag-track/SKILL.md").contains("name: tag-track\n"));
    assert!(cli::SKILL.contains("/skills/tag-track\n"));
}

#[test]
fn native_artwork_path_rejects_json_before_any_output_commit() {
    use std::os::unix::ffi::OsStringExt;
    let tmp = temp();
    let source = tmp
        .path()
        .join(std::ffi::OsString::from_vec(b"native-\xff.flac".to_vec()));
    let output = binary()
        .env("TAG_TRACK_PROGRESS_TARGET", "stdout")
        .args(["artwork", "embed", "--artwork", "missing.png"])
        .arg(&source)
        .arg("--out-dir")
        .arg(tmp.path().join("out"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires UTF-8"));
    assert!(!tmp.path().join("out").exists());
    let output = binary().args(["dependencies", "check"]).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("ffprobe: OK"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_current_release_channel_runs_native_self_updater_offline() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().fallback(axum::routing::any(|| async {
        (
            axum::http::StatusCode::FOUND,
            [("location", "/owner/repo/releases/tag/v0.1.0")],
        )
    }));
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let output = binary()
        .env(
            "TAG_TRACK_RUST_RELEASE_BASE_URL",
            format!("http://{address}"),
        )
        .env("TAG_TRACK_RUST_RELEASE_OWNER", "owner")
        .env("TAG_TRACK_RUST_RELEASE_REPO", "repo")
        .args(["self", "update"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"update: current\nversion: 0.1.0\n");
    server.abort();
}
