use std::println;

use clap::{Parser, Subcommand};
use felis::{
    command::{self, Echo, OpenInHelix},
    server::executor::Flag,
    Result,
};
use felis_command::{ReadWire, WriteWire};
use tokio::{io::AsyncWriteExt, net::UnixStream};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    Echo { message: Vec<String> },
    OpenInHelix { tab_id: u8, path: String },
    Shutdown,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut socket = UnixStream::connect("/tmp/felis.sock").await?;

    match cli.command {
        Command::Echo { message } => {
            let message = message.join(" ");

            Echo { message }.write(&mut socket).await?;

            let response = String::read(&mut socket).await?;
            println!("{response}");
        }
        Command::OpenInHelix { tab_id, path } => {
            let flag = if cli.dry_run {
                Flag::DryRun
            } else {
                Flag::NoOp
            };

            let cmd = OpenInHelix {
                flag,
                kitty_tab_id: tab_id,
                path,
            };
            cmd.write(&mut socket).await?;

            let response = String::read(&mut socket).await?;
            println!("{response}");
        }
        Command::Shutdown => {
            socket.write_u8(command::SHUTDOWN).await?;
        }
    }

    Ok(())
}
