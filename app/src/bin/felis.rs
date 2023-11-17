use std::{path::PathBuf, println, process::Stdio};

use clap::{Parser, Subcommand};
use felis::Result;
use felis_protocol::{WireRead, WireWrite};
use tokio::{io::AsyncReadExt, net::UnixStream};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Echo {
        message: Vec<String>,
    },
    GetActiveFocusedWindow,
    OpenInHelix {
        #[arg(short, long, required_unless_present("file_browser"))]
        path: Option<PathBuf>,
        #[arg(short, long)]
        tab_id: Option<u32>,
        #[arg(short, long, required_unless_present("path"))]
        file_browser: Option<String>,
    },
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
        }
        Command::GetActiveFocusedWindow => {
            felis::Command::GetActiveFocusedWindow
                .write(&mut socket)
                .await?;
        }
        Command::OpenInHelix {
            tab_id,
            path,
            file_browser,
        } => {
            let path = if let Some(path) = path {
                path
            } else if let Some(file_browser) = file_browser {
                let mut child = tokio::process::Command::new(file_browser)
                    .stdout(Stdio::piped())
                    .spawn()?;
                let mut stdout = child.stdout.take().unwrap();
                let mut out = String::new();
                stdout.read_to_string(&mut out).await?;
                let out = out.trim_end();
                PathBuf::from(out)
            } else {
                panic!("This shouldn't happen")
            };

            println!("path: {path:?}");

            let cmd = felis::Command::OpenInHelix {
                kitty_tab_id: tab_id,
                path,
            };
            cmd.write(&mut socket).await?;
        }
        Command::Shutdown => {
            felis::Command::Shutdown.write(&mut socket).await?;
        }
    }

    let response = felis::Response::read(&mut socket).await?;

    match *response {
        felis::Response::Ack => (),
        felis::Response::Message(msg) => println!("{msg}"),
        felis::Response::WindowId(id) => println!("{id}"),
    }

    Ok(())
}
