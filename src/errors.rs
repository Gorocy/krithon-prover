use crate::request::Rule as RequestRule;
use crate::response::Rule as ResponseRule;
use hyper::Error as HyperError;
use pest::error::Error as PestError;
use std::io;
use thiserror::Error;
use tlsn_common::config::ProtocolConfigBuilderError;
use tlsn_prover::{ProverConfigBuilderError, ProverError};
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, Errors>;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("Failed to read size from extension")]
    FailedToReadSizeFromExtension,
    
    #[error("Failed to read message from extension")]
    FailedToReadMessageFromExtension,

    #[error("Invalid scheme")]
    InvalidScheme,

    #[error("Request URI does not have an authority or host")]
    MissingAuthority,

    #[error("Request URI does not have a port")]
    MissingPort,

    #[error(transparent)]
    Utf8ConversionError(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    ProverConfigBuilderError(#[from] ProverConfigBuilderError),

    #[error(transparent)]
    ProverError(#[from] ProverError),

    #[error(transparent)]
    ProtocolConfigBuilderError(#[from] ProtocolConfigBuilderError),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    HyperError(#[from] HyperError),

    #[error(transparent)]
    JoinError(#[from] JoinError),

    #[error(transparent)]
    PestRequestError(#[from] PestError<RequestRule>),

    #[error(transparent)]
    PestResponseError(#[from] PestError<ResponseRule>),

    #[error("{0}")]
    StringError(String),


} 