use thiserror::Error;

pub type Result<T> = std::result::Result<T, Errors>;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("Failed to read size from extension")]
    FailedToReadSizeFromExtension,
    #[error("Failed to read message from extension")]
    FailedToReadMessageFromExtension,
} 