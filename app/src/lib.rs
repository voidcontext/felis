pub mod server;

use crate::server::executor::Flag;
use felis_protocol::{WireReadError, WireWriteError};
use felis_protocol_macro::{WireRead, WireWrite};
use std::{io::Error, num::TryFromIntError, string::FromUtf8Error};
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
}

#[derive(WireRead, WireWrite)]
pub enum Command {
    Shutdown,
    Echo(String),
    OpenInHelix {
        flag: Flag,
        kitty_tab_id: u8,
        path: String,
    },
}
