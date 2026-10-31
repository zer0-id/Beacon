use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // Protocol
    #[error("")] // TODO => Provide a proper error message
    PostcardError(#[from] postcard::Error),

    #[error("The message is larger than 5MB")]
    MessageTooLargeForFrame,
    #[error("The frame length is too small")]
    FramelengthMistmatch,
    #[error("The message is not defined in the Beacon protocol")]
    UnrecognizedMessage,
    #[error("The frame received is smaller than 4 bytes")]
    InvalidFrameLength,

    // Vault (Database)
    #[error("sqlite error")]
    HarbourError(#[from] rusqlite::Error),
}