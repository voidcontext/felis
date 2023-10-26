use std::println;

use clap::{Parser, Subcommand};
use felis::{
    command,
    util::{ReadPayloadExt, WritePayloadExt},
    Result,
};
use tokio::{io::AsyncWriteExt, net::UnixStream};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Echo { message: Vec<String> },
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
        Command::Shutdown => {
            socket.write_u8(command::SHUTDOWN).await?;
        }
    }

    Ok(())
}
