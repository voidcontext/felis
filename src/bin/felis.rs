use std::{
    path::{Path, PathBuf},
    println,
    process::Stdio,
};

use clap::{Parser, Subcommand};
use felis::{
    command, fs::AbsolutePath, kitty_terminal::KittyTerminal, Context, Environment, Result,
};
use kitty_remote_bindings::{
    command::options::{Cwd, LaunchType},
    model::WindowId,
};
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
        window_id: Option<u32>, // TODO: change this to Option<WindowId>
        /// The context of how felis is used, this drives how file paths are determined
        #[arg(long, default_value_t = Context::Shell)]
        context: Context,
        /// Wether to use the steel plugin to open the file
        #[arg(long, default_value_t = false)]
        steel: bool,
    },
    /// Run the given file browser / file manager and then open the selected file in helix
    OpenBrowser {
        /// Name or path to the executable to run to select the file to open. The given program
        /// needs to print the path of then file to the standard output, e.g. a propertly configured
        /// `broot`
        file_browser: String,
        /// Sets the current working directory to the given path when opens the file browser
        cwd: Option<PathBuf>,
        /// Open the file in the helix process running in the given window. If not given felis will
        /// try to determine which helix instance is running in one the parent directories of the
        /// given file.
        #[arg(short, long)]
        window_id: Option<u32>, // TODO:  change this to Option<WindowId>
        /// When true felis will launch a kitty overlay on top the current window, and run the file
        /// browser there. This is useful when felis is running from an editor.
        #[arg(short, long, default_value_t = false)]
        launch_overlay: bool,
        /// Wether to use the steel plugin to open the file
        #[arg(long, default_value_t = false)]
        steel: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let kitty = KittyTerminal::new(kitty_socket()?);

    match cli.command {
        Command::GetActiveFocusedWindow => {
            let window_id = command::get_active_focused_window(&kitty).await?;
            println!("{}", window_id.0);
        }

        Command::OpenFile {
            path,
            window_id,
            context,
            steel,
        } => {
            let env = env(&context, &kitty).await?;
            let path = AbsolutePath::resolve(&path, &env)?;
            command::open_in_helix(&path, window_id.map(WindowId), &kitty, steel).await?;
        }

        Command::OpenBrowser {
            file_browser,
            window_id,
            launch_overlay,
            steel,
            cwd,
        } => {
            if launch_overlay {
                let executable = std::env::current_exe()?;

                let mut args = vec![
                    executable.as_os_str().to_str().unwrap().to_string(),
                    "open-browser".to_string(),
                    file_browser.as_str().to_string(),
                ];

                if let Some(tab_id) = window_id {
                    args.push("--tab-id".to_string());
                    args.push(tab_id.to_string());
                };

                if steel {
                    args.push("--steel".to_string());
                }

                if let Some(dir) = &cwd {
                    args.push(dir.to_string_lossy().to_string());
                }

                let working_dir = if let Some(dir) = cwd {
                    Cwd::Path(dir)
                } else {
                    Cwd::Current
                };

                // TODO: replace this with a KittyComman
                kitty.launch(args, LaunchType::Overlay, working_dir).await?;
            } else {
                let mut cmd = tokio::process::Command::new(file_browser);

                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }

                let mut child = cmd.stdout(Stdio::piped()).spawn()?;

                let mut stdout = child.stdout.take().unwrap();
                let mut out = String::new();
                stdout.read_to_string(&mut out).await?;
                let out = out.trim_end();
                let path = PathBuf::from(out);

                let path = AbsolutePath::try_from(path)?;

                command::open_in_helix(&path, window_id.map(WindowId), &kitty, steel).await?;
            }
        }
    };

    Ok(())
}

// When sockets are enabled the KITTY_LISTEN_ON env var is set in shells running in kitty windows.
// But when felis is executed via `pass_selection_to_program`, then the env var is not set and the
// kitty program spawned by felis cannot communicate through tty either, so we need this heuristic
// here when we try to find the socket file based on the parent's p, so we need this heuristic here
// when we try to find the socket file based on the parent's pid
fn kitty_socket() -> Result<String> {
    if let Ok(socket) = std::env::var("KITTY_LISTEN_ON") {
        Ok(socket)
    } else {
        let parent_pid = std::os::unix::process::parent_id();
        let socket = format!("/tmp/kitty.sock-{parent_pid}");
        if Path::new(&socket).exists() {
            Ok(format!("unix:{socket}"))
        } else {
            Err(felis::FelisError::UnexpectedError {
                message: "couldn't determine kitty socket".to_string(),
            })
        }
    }
}

async fn env(context: &Context, terminal: &KittyTerminal) -> Result<Environment> {
    match context {
        Context::Shell => {
            let cwd = std::env::current_dir()?;
            Ok(Environment::Shell(cwd))
        }
        Context::Terminal => {
            let windows = terminal.ls().await?;
            Ok(Environment::Kitty(windows))
        }
    }
}
