use thiserror::Error;

use starknet::{
    accounts::{single_owner::SignError, AccountError},
    core::{
        types::{FromByteSliceError, FromStrError},
        utils::NonAsciiNameError,
    },
    providers::ProviderError,
};

/// Represents errors that can occur when handling operations within the application.
#[derive(Debug, Error)]
pub enum HandlerError {
    /// Represents an error from the provider.
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    /// Represents an error from the account.
    #[error("Account error: {0}")]
    AccountError(#[from] AccountError<SignError<starknet::signers::local_wallet::SignError>>),

    /// Represents a parse error for field elements.
    #[error("Parse error")]
    ParseError(#[from] FieldElementParseError),

    /// Represents an error for non-ASCII names.
    #[error("Non Ascii Name: {0}")]
    NonAsciiName(#[from] NonAsciiNameError),
}

/// Represents errors that can occur when parsing field elements.
#[derive(Debug, Error)]
pub enum FieldElementParseError {
    /// Represents an error that occurs when parsing from a string.
    #[error("FromStr error: {0}")]
    FromStrError(#[from] FromStrError),

    /// Represents an error that occurs when parsing from a byte slice.
    #[error("FromByteSlice error: {0}")]
    FromByteSliceError(#[from] FromByteSliceError),
}
