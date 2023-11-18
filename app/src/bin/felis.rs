use std::{path::PathBuf, println, process::Stdio};

use clap::{Parser, Subcommand};
use felis::{kitty_terminal::KittyTerminal, server::handle_command, Result};
use tokio::io::AsyncReadExt;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    GetActiveFocusedWindow,
    OpenInHelix {
        #[arg(short, long, required_unless_present("file_browser"))]
        path: Option<PathBuf>,
        #[arg(short, long)]
        tab_id: Option<u32>,
        #[arg(short, long, required_unless_present("path"))]
        file_browser: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let kitty = KittyTerminal::new();
    let response = match cli.command {
        Command::GetActiveFocusedWindow => {
            handle_command(&felis::Command::GetActiveFocusedWindow, &kitty).await?
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

            handle_command(
                &felis::Command::OpenInHelix {
                    kitty_tab_id: tab_id,
                    path,
                },
                &kitty,
            )
            .await?
        }
    };

    match response {
        felis::Response::Ack => (),
        felis::Response::Message(msg) => println!("{msg}"),
        felis::Response::WindowId(id) => println!("{id}"),
    }

    Ok(())
}
