use std::path::{Path, PathBuf};

use kitty_remote_bindings::{
    model::{self, OsWindows, Window, WindowId},
    Matcher,
};

use crate::{kitty_terminal::KittyTerminal, Command, FelisError, Response, Result};

/// # Errors
///
/// Will return Err if Kitty terminal related operations fail
pub async fn handle_command(cmd: &Command, kitty: &KittyTerminal) -> Result<Response> {
    let response = match &cmd {
        Command::GetActiveFocusedWindow => {
            let windows = kitty.ls().await?;
            let window =
                focused_active_window(&windows).ok_or_else(|| FelisError::UnexpectedError {
                    message: "Couldn't find active focused window".to_string(),
                })?;

            Response::WindowId(window.id.0)
        }
        Command::OpenInHelix { path, kitty_tab_id } => {
            let windows = kitty.ls().await?;
            let kitty_window = if let Some(id) = kitty_tab_id {
                find_window_by_id(&windows, WindowId(*id)).ok_or_else(|| {
                    FelisError::UnexpectedError {
                        message: format!("Couldn't find window with id {id}"),
                    }
                })?
            } else {
                find_workspace(&windows, path)?
            };

            let rel_path = if path.is_absolute() {
                path.strip_prefix(window_cwd(kitty_window))?
            } else {
                path
            };

            kitty.focus_window(Matcher::Id(kitty_window.id)).await?;
            kitty.send_text(Matcher::Id(kitty_window.id), r"\E").await?;
            kitty
                .send_text(
                    Matcher::Id(kitty_window.id),
                    format!(r":open {}\r", rel_path.to_string_lossy()).as_str(),
                )
                .await?;

            Response::Ack
        }
    };

    Ok(response)
}

fn find_window_by_id(windows: &OsWindows, window_id: WindowId) -> Option<&Window> {
    windows.0.iter().find_map(|os_window| {
        os_window
            .tabs
            .iter()
            .find_map(|tab| tab.windows.iter().find(|window| window.id == window_id))
    })
}

fn focused_active_window(windows: &OsWindows) -> Option<&model::Window> {
    windows
        .0
        .iter()
        .filter(|window| window.is_active && window.is_focused)
        .find_map(|os_window| {
            os_window
                .tabs
                .iter()
                .filter(|tab| tab.is_active && tab.is_focused)
                .find_map(|tab| tab.windows.iter().find(|w| w.is_active && w.is_focused))
        })
}

fn window_cwd(window: &Window) -> &Path {
    window.foreground_processes[0].cwd.as_path()
}

fn resolve_relative_path(windows: &OsWindows, path: &Path) -> PathBuf {
    // Resolving the relative path needs to happen by trying to find the active window, and get the
    // working directory from its first process. We cannot just get it from the client (the felis
    // program working dir), because it might not be runnnig from a shell context, it might be
    // executed by kitty (e.g. when using `pass_selection_to_program`).
    //
    // This could fail badly if the selection is not in the active, focused window.
    if path.is_relative() {
        if let Some(window) = focused_active_window(windows) {
            let mut path_buf = PathBuf::new();
            path_buf.push(window_cwd(window));
            path_buf.push(path);
            path_buf
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}

fn find_workspace<'a>(windows: &'a OsWindows, path: &Path) -> Result<&'a Window> {
    let path = resolve_relative_path(windows, path);

    let workspace_window = windows.0.iter().find_map(|os_window| {
        os_window.tabs.iter().find_map(|tab| {
            tab.windows.iter().find(|w| {
                w.foreground_processes
                    .iter()
                    .any(|process| is_helix_bin(process) && is_in_workspace(process, &path))
            })
        })
    });

    workspace_window.ok_or_else(|| FelisError::UnexpectedError {
        message: format!("Couldn't find workspace for file {path:?}"),
    })
}

fn is_in_workspace(process: &model::Process, path: &Path) -> bool {
    path.parent()
        .map_or(false, |p| p.starts_with(process.cwd.as_path()))
}

fn is_helix_bin(process: &model::Process) -> bool {
    process.cmdline.iter().any(|c| c.ends_with("bin/hx"))
}

#[cfg(test)]
mod test {

    use std::{
        os::unix::process::ExitStatusExt,
        path::PathBuf,
        process::{ExitStatus, Output},
    };

    use kitty_remote_bindings::{model::WindowId, FocusWindow, Ls, Matcher, MatcherExt, SendText};
    use mockall::predicate::*;
    use pretty_assertions::assert_eq;

    use crate::{
        kitty_terminal::{test_fixture, KittyTerminal, MockExecutor},
        server::command_server::handle_command,
        Command, Response,
    };

    fn expect_ls_success(executor: &mut MockExecutor) {
        executor
            .expect_ls()
            .times(1)
            .with(eq(Ls::new()))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
    }

    fn expect_send_text_success(executor: &mut MockExecutor, text: &str, window_id: WindowId) {
        let mut cmd = SendText::new(text.to_string());
        cmd.matcher(Matcher::Id(window_id));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });
    }

    fn expect_focus_window_succes(executor: &mut MockExecutor, window_id: WindowId) {
        let mut cmd = FocusWindow::new();
        cmd.matcher(Matcher::Id(window_id));
        executor
            .expect_focus_window()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });
    }

    #[tokio::test]
    async fn test_get_active_focused_window_command() {
        let cmd = Command::GetActiveFocusedWindow;

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);

        let response = handle_command(&cmd, &KittyTerminal::mock(executor))
            .await
            .unwrap();
        assert_eq!(response, Response::WindowId(2));
    }

    #[tokio::test]
    async fn test_open_in_helix_with_kitty_tab_id_command() {
        let path = "src/lib.rs";

        let cmd = Command::OpenInHelix {
            kitty_tab_id: Some(1),
            path: PathBuf::from(path),
        };
        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_focus_window_succes(&mut executor, WindowId(1));
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(
            &mut executor,
            format!(r":open {path}\r").as_str(),
            WindowId(1),
        );

        let response = handle_command(&cmd, &KittyTerminal::mock(executor))
            .await
            .unwrap();
        assert_eq!(response, Response::Ack);
    }

    #[tokio::test]
    async fn test_open_in_helix_turns_absolute_path_to_relative() {
        let path = "/path/to/felis/src/lib.rs";

        let cmd = Command::OpenInHelix {
            kitty_tab_id: Some(1),
            path: PathBuf::from(path),
        };

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_focus_window_succes(&mut executor, WindowId(1));
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(&mut executor, r":open src/lib.rs\r", WindowId(1));

        let response = handle_command(&cmd, &KittyTerminal::mock(executor))
            .await
            .unwrap();
        assert_eq!(response, Response::Ack);
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id_command() {
        let path = "src/lib.rs";

        let cmd = Command::OpenInHelix {
            kitty_tab_id: None,
            path: PathBuf::from(path),
        };

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(
            &mut executor,
            format!(r":open {path}\r").as_str(),
            WindowId(1),
        );
        expect_focus_window_succes(&mut executor, WindowId(1));

        let response = handle_command(&cmd, &KittyTerminal::mock(executor))
            .await
            .unwrap();

        assert_eq!(response, Response::Ack);
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id_command_resolves_relative_path() {
        let path = "src/lib.rs";

        let cmd = Command::OpenInHelix {
            kitty_tab_id: None,
            path: PathBuf::from(path),
        };

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(
            &mut executor,
            format!(r":open {path}\r").as_str(),
            WindowId(1),
        );
        expect_focus_window_succes(&mut executor, WindowId(1));

        let response = handle_command(&cmd, &KittyTerminal::mock(executor))
            .await
            .unwrap();
        assert_eq!(response, Response::Ack);
    }
}
