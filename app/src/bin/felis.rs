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
    /// Open the given file in helix
    OpenFile {
        /// Path to the file to open
        path: PathBuf,
        /// Open the file in the helix process running in the given window
        #[arg(short, long)]
        tab_id: Option<u32>,
    },
    /// Run the given file browser / file manager and then open the selected file in helix
    OpenBrowser {
        /// Name or path to the executable to run to select the file to open. The given program
        /// needs to print the path of then file to the standard output, e.g. a propertly configured
        /// `broot`
        file_browser: String,
        /// Open the file in the helix process running in the given window. If not given felis will
        /// try to determine which helix instance is running in one the parent directories of the
        /// given file.
        #[arg(short, long)]
        tab_id: Option<u32>,
        /// When true felis will launch a kitty overlay on top the current window, and run the file
        /// browser there. This is useful when felis is running from an editor.
        #[arg(short, long, default_value_t = false)]
        launch_overlay: bool,
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
        Command::OpenFile { path, tab_id } => {
            handle_command(
                &felis::Command::OpenInHelix {
                    kitty_tab_id: tab_id,
                    path,
                },
                &kitty,
            )
            .await?
        }
        Command::OpenBrowser {
            file_browser,
            tab_id,
            launch_overlay,
        } => {
            if launch_overlay {
                let executable = std::env::current_exe()?;

                // TODO: replace this with a KittyCommand
                let mut cmd = tokio::process::Command::new("kitty");
                cmd.args([
                    "@",
                    "launch",
                    "--type",
                    "overlay",
                    "--cwd",
                    "current",
                    executable.as_os_str().to_str().unwrap(),
                    "open-browser",
                    file_browser.as_str(),
                ]);

                if let Some(tab_id) = tab_id {
                    cmd.arg("--tab-id");
                    cmd.arg(tab_id.to_string());
                }

                cmd.spawn()?.wait().await?;

                felis::Response::Ack
            } else {
                let mut child = tokio::process::Command::new(file_browser)
                    .stdout(Stdio::piped())
                    .spawn()?;
                let mut stdout = child.stdout.take().unwrap();
                let mut out = String::new();
                stdout.read_to_string(&mut out).await?;
                let out = out.trim_end();
                let path = PathBuf::from(out);

                handle_command(
                    &felis::Command::OpenInHelix {
                        kitty_tab_id: tab_id,
                        path,
                    },
                    &kitty,
                )
                .await?;

                felis::Response::Ack
            }
        }
    };

    match response {
        felis::Response::Ack => (),
        felis::Response::Message(msg) => println!("{msg}"),
        felis::Response::WindowId(id) => println!("{id}"),
    }

    Ok(())
}
