pub mod kitty_terminal;
pub mod server;

use felis_protocol::{WireReadError, WireWriteError};
use felis_protocol_macro::{WireRead, WireWrite};
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
    #[error("WireFormatRead error")]
    WireFormatRead(#[from] WireReadError),
    #[error("WireFormatWrite error")]
    WireFormatWrite(#[from] WireWriteError),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
    #[error("kitty error")]
    KittyError(#[from] kitty_remote_bindings::Error),
    #[error("strip prefix error")]
    StripPrefixError(#[from] StripPrefixError),
}

#[derive(WireRead, WireWrite)]
pub enum Command {
    Shutdown,
    Echo(String),
    GetActiveFocusedWindow,
    OpenInHelix {
        path: PathBuf,
        kitty_tab_id: Option<u32>,
    },
}

#[derive(Debug, WireRead, WireWrite, PartialEq)]
pub enum Response {
    Ack,
    Message(String),
    WindowId(u32),
}
