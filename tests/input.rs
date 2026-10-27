use std::{
    fs::File,
    io::Write,
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};
use tag_track_cli::{CancellationToken, input};
fn pipe() -> (File, File) {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    unsafe {
        (
            File::from(OwnedFd::from_raw_fd(fds[0])),
            File::from(OwnedFd::from_raw_fd(fds[1])),
        )
    }
}
fn flags(file: &File) -> i32 {
    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) }
}
#[tokio::test]
async fn native_input_reads_null_regular_and_pipe_and_restores_flags() {
    let token = CancellationToken::new();
    let null = File::open("/dev/null").unwrap();
    assert_eq!(
        input::read_descriptor(null.as_fd(), &token).await.unwrap(),
        ""
    );
    let (read, mut write) = pipe();
    let original = flags(&read);
    write.write_all(b"DONE: /music/song.flac\n").unwrap();
    drop(write);
    assert_eq!(
        input::read_descriptor(read.as_fd(), &token).await.unwrap(),
        "DONE: /music/song.flac\n"
    );
    assert_eq!(flags(&read), original);
    let mut file = tempfile::tempfile_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    file.write_all(&[255]).unwrap();
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).unwrap();
    assert!(
        input::read_descriptor(file.as_fd(), &token)
            .await
            .unwrap_err()
            .to_string()
            .contains("UTF-8")
    );
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    assert!(
        input::read_descriptor(file.as_fd(), &token)
            .await
            .unwrap_err()
            .to_string()
            .contains("8 MiB")
    );
}
#[tokio::test]
async fn pending_pipe_cancellation_restores_flags_and_drops_cleanly() {
    let (read, _write) = pipe();
    let original = flags(&read);
    let token = CancellationToken::new();
    let cancel = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        cancel.cancel()
    });
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        input::read_descriptor(read.as_fd(), &token),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(tag_track_cli::exit_code(&error), 130);
    assert_eq!(flags(&read), original);
}

#[tokio::test]
async fn readiness_reads_later_bytes_and_write_only_input_errors_restore_flags() {
    let (read, mut write) = pipe();
    let original = flags(&read);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        write.write_all(b"song.flac\n").unwrap();
    });
    assert_eq!(
        input::read_descriptor(read.as_fd(), &CancellationToken::new())
            .await
            .unwrap(),
        "song.flac\n"
    );
    assert_eq!(flags(&read), original);
    let (_read, write) = pipe();
    let original = flags(&write);
    assert!(
        input::read_descriptor(write.as_fd(), &CancellationToken::new())
            .await
            .is_err()
    );
    assert_eq!(flags(&write), original);
}
#[test]
fn line_limits_home_and_empty_result_lines() {
    assert!(input::files_from_lines(&"x".repeat(65537)).is_err());
    assert!(
        input::files_from_lines("DONE: \noutput: \n")
            .unwrap()
            .is_empty()
    );
    let home = dirs::home_dir().unwrap();
    assert_eq!(input::expand_home("~".as_ref()), home);
    assert_eq!(input::expand_home("~/music".as_ref()), home.join("music"));
    assert_eq!(
        input::expand_home("relative".as_ref()),
        std::path::PathBuf::from("relative")
    );
}
