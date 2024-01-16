pub mod command;
pub mod fs;
pub mod kitty_terminal;

use clap::ValueEnum;
use kitty_remote_bindings::model::OsWindows;
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

impl From<String> for FelisError {
    fn from(value: String) -> Self {
        Self::UnexpectedError { message: value }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Context {
    Shell,
    Terminal,
}

impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = format!("{self:?}").to_lowercase();
        f.write_str(value.as_str())
    }
}

pub enum Environment {
    Shell(PathBuf), // cwd
    Kitty(OsWindows),
}
