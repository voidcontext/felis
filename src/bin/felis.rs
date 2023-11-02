use std::println;

use clap::{Parser, Subcommand};
use felis::{
    command,
    server::executor::Flag,
    util::{ReadPayloadExt, WritePayloadExt},
    Result,
};
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
            socket.write_u8(felis::command::ECHO).await?;
            socket.write_payload(message.as_bytes()).await?;

            let response = socket.read_payload().await?;
            println!("{}", String::from_utf8(response)?);
        }
        Command::OpenInHelix { tab_id, path } => {
            socket.write_u8(felis::command::OPEN_IN_HELIX).await?;
            socket
                .write_u8(if cli.dry_run {
                    Flag::DryRun as u8
                } else {
                    0_u8
                })
                .await?;
            socket.write_u8(tab_id).await?;
            socket.write_payload(path.as_bytes()).await?;

            let response = socket.read_payload().await?;
            println!("{}", String::from_utf8(response)?);
        }
        Command::Shutdown => {
            socket.write_u8(command::SHUTDOWN).await?;
        }
    }

    Ok(())
}
