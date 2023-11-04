use felis_command::{AsyncRead, AsyncWrite, Command};
use felis_command_macro::command;

use crate::server::executor::Flag;

pub const SHUTDOWN: u8 = 10;
pub const ECHO: u8 = 11;
pub const OPEN_IN_HELIX: u8 = 12;

#[command(SHUTDOWN)]
pub struct Shutdown;

#[command(ECHO)]
pub struct Echo {
    pub message: String,
}

#[command(OPEN_IN_HELIX)]
pub struct OpenInHelix {
    pub flag: Flag,
    pub kitty_tab_id: u8,
    pub path: String,
}
