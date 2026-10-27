pub mod cli;
pub mod input;
pub mod progress;
pub mod workflow;
pub use audio_metadata::CancellationToken;
pub use workflow::{Options, Outcome, Workflow};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Batch(String),
    #[error(transparent)]
    Audio(#[from] audio_metadata::Error),
    #[error(transparent)]
    Progress(#[from] cli_event_stream::Error),
    #[error(transparent)]
    Dependency(#[from] cli_deps::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

pub fn exit_code(error: &Error) -> u8 {
    match error {
        Error::Audio(audio_metadata::Error::Cancelled)
        | Error::Progress(cli_event_stream::Error::Cancelled)
        | Error::Dependency(cli_deps::Error::Cancelled) => 130,
        _ => 1,
    }
}
