pub mod kitty_terminal;
pub mod server;

use std::{
    io::Error,
    num::TryFromIntError,
    path::{PathBuf, StripPrefixError},
    string::FromUtf8Error,
};
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

pub enum Command {
    GetActiveFocusedWindow,
    OpenInHelix {
        path: PathBuf,
        kitty_tab_id: Option<u32>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Response {
    Ack,
    Message(String),
    WindowId(u32),
}
