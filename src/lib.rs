pub mod command;
pub mod kitty_terminal;

use std::{io::Error, num::TryFromIntError, path::StripPrefixError, string::FromUtf8Error};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FelisError>;

#[derive(Error, Debug)]
pub enum FelisError {
    #[error("I/O error")]
    IO(#[from] Error),
    #[error("TryFromInt error")]
    TryFromInt(#[from] TryFromIntError),
    #[error("FromUtf8 error")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
    #[error("kitty error")]
    KittyError(#[from] kitty_remote_bindings::Error),
    #[error("strip prefix error")]
    StripPrefixError(#[from] StripPrefixError),
}
