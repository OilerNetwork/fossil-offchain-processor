use thiserror::Error;

use starknet::{
    accounts::{single_owner::SignError, AccountError},
    core::{types::eth_address::FromBytesSliceError, utils::NonAsciiNameError},
    providers::ProviderError,
};
use starknet_types_core::felt::FromStrError;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),
    #[error("Account error: {0}")]
    AccountError(#[from] AccountError<SignError<starknet::signers::local_wallet::SignError>>),
    #[error("Parse error")]
    ParseError(#[from] FieldElementParseError),
    #[error("Non Ascii Name: {0}")]
    NonAsciiName(#[from] NonAsciiNameError),
}

#[derive(Debug, Error)]
pub enum FieldElementParseError {
    #[error("FromStr error: {0}")]
    FromStrError(#[from] FromStrError),
    #[error("FromByteSlice error: {0}")]
    FromByteSliceError(#[from] FromBytesSliceError),
}

