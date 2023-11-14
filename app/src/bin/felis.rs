use std::{path::PathBuf, println};

use clap::{Parser, Subcommand};
use felis::Result;
use felis_protocol::{WireRead, WireWrite};
use tokio::net::UnixStream;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Echo { message: Vec<String> },
    OpenInHelix { path: PathBuf, tab_id: Option<u32> },
    Shutdown,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut socket = UnixStream::connect("/tmp/felis.sock").await?;

    match cli.command {
        Command::Echo { message } => {
            let message = message.join(" ");

            felis::Command::Echo(message).write(&mut socket).await?;

            let response = String::read(&mut socket).await?;
            println!("{response}");
        }
        Command::OpenInHelix { tab_id, path } => {
            let cmd = felis::Command::OpenInHelix {
                kitty_tab_id: tab_id,
                path,
            };
            cmd.write(&mut socket).await?;

            let response = String::read(&mut socket).await?;
            println!("{response}");
        }
        Command::Shutdown => {
            felis::Command::Shutdown.write(&mut socket).await?;
        }
    }

    Ok(())
}
