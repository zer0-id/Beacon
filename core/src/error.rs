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

    // Path Validator
    /// If path is empty or contains many '/'.
    #[error("The Path cannot be empty")]
    Empty,
    /// Path traversal rejection.
    #[error("The Path cannot contain a dot")]
    Dot,
    /// Path traversal rejection.
    #[error("The Path cannot contain two dots")]
    DotDot,
    /// Directory/File name in path exceeds MAX_NAME_BYTES.
    #[error("Name of a file or directory in the path is too long")]
    NameTooLong { bytes: usize },
    /// Path size exceeds MAX_PATH_BYTES.
    #[error("The Path is too long")]
    PathTooLong { bytes: usize },
    /// Path contains an invalid char, check FORBIDDEN_CHARS.
    #[error("The Path contains an invalid character")]
    InvalidChar(char),
}

