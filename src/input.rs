use crate::{Error, Result};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

static VIDEO_ID: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\s*\[[a-zA-Z0-9_-]{11}\]\s*").expect("valid video ID regex")
});

/// Keep descriptive brackets in search queries while removing source video IDs.
pub fn search_name(raw: &str) -> String {
    VIDEO_ID
        .replace_all(raw, " ")
        .chars()
        .map(|c| match c {
            '–' | '—' | '−' | '‐' | '‑' | '‒' | '―' => '-',
            '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>' | '`' => '_',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn expand_home(path: &Path) -> PathBuf {
    if path == Path::new("~") {
        return dirs::home_dir().unwrap_or_else(|| path.to_owned());
    }
    if let Ok(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    path.to_owned()
}
pub fn files_from_lines(raw: &str) -> Result<Vec<PathBuf>> {
    let mut plain = Vec::new();
    let mut results = Vec::new();
    for line in raw.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if line.len() > 65536 {
            return Err(Error::Invalid("input path line exceeds 64 KiB".into()));
        }
        if matches!(line, "DONE:" | "output:") {
            continue;
        }
        if let Some(path) = line
            .strip_prefix("DONE: ")
            .or_else(|| line.strip_prefix("output: "))
        {
            let path = path.trim();
            if !path.is_empty() {
                results.push(expand_home(Path::new(path)));
            }
        } else {
            plain.push(expand_home(Path::new(line)));
        }
    }
    Ok(if results.is_empty() { plain } else { results })
}

/// Read the designated input descriptor exclusively until EOF or cancellation.
/// Its caller retains ownership; O_NONBLOCK is restored on every return path.
pub async fn read_descriptor(
    fd: std::os::fd::BorrowedFd<'_>,
    token: &crate::CancellationToken,
) -> Result<String> {
    use std::{
        io::{self, Read},
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
    };
    audio_metadata::check_cancel(token)?;
    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: fcntl returned a fresh descriptor, owned only by this File.
    let file = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(duplicate) });
    const LIMIT: usize = 8 * 1024 * 1024;
    let mut bytes = Vec::new();
    if file.metadata()?.is_file() {
        file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    } else {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error().into());
        }
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error().into());
        }
        let mut owned = InputDescriptor { file, flags };
        // Read immediately before registering: native descriptors such as /dev/null
        // reach EOF but cannot be registered with epoll. O_NONBLOCK bounds reads.
        loop {
            audio_metadata::check_cancel(token)?;
            let mut buffer = [0; 8192];
            match owned.file.read(&mut buffer) {
                Ok(0) => return decode(bytes),
                Ok(count) => {
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > LIMIT {
                        return decode(bytes);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        // SAFETY: the stable owned descriptor remains alive until deregistration.
        let input =
            unsafe { tokio::io::unix::AsyncFd::register(owned) }.map_err(io::Error::from)?;
        loop {
            let mut ready = tokio::select! {biased;_=token.cancelled()=>return Err(audio_metadata::Error::Cancelled.into()),r=input.readable()=>r?};
            let mut buffer = [0; 8192];
            match ready.try_io(|fd| (&fd.get_ref().file).read(&mut buffer)) {
                Ok(Ok(0)) => break,
                Ok(Ok(count)) => {
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > LIMIT {
                        break;
                    }
                }
                Ok(Err(e)) if e.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => continue,
            }
        }
    }
    decode(bytes)
}
fn decode(bytes: Vec<u8>) -> Result<String> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Error::Invalid("stdin exceeds 8 MiB".into()));
    }
    String::from_utf8(bytes).map_err(|_| {
        Error::Invalid("stdin paths must use UTF-8; pass native paths as file arguments".into())
    })
}
struct InputDescriptor {
    file: std::fs::File,
    flags: i32,
}
impl std::os::fd::AsRawFd for InputDescriptor {
    fn as_raw_fd(&self) -> i32 {
        self.file.as_raw_fd()
    }
}
impl Drop for InputDescriptor {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        unsafe { libc::fcntl(self.as_raw_fd(), libc::F_SETFL, self.flags) };
    }
}
pub async fn read_stdin(token: &crate::CancellationToken) -> Result<String> {
    // SAFETY: the standard input descriptor is borrowed only for this call.
    read_descriptor(
        unsafe { std::os::fd::BorrowedFd::borrow_raw(libc::STDIN_FILENO) },
        token,
    )
    .await
}
