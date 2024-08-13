use std::path::Path;

use kitty_remote_bindings::{
    command::options::Matcher,
    model::{self, OsWindows, Window, WindowId},
};

use crate::{fs::AbsolutePath, kitty_terminal::KittyTerminal, FelisError, Result};

/// # Errors
///
/// Will return Err if Kitty terminal related operations fail
pub async fn get_active_focused_window(kitty: &KittyTerminal) -> Result<WindowId> {
    let windows = kitty.ls().await?;
    let window = focused_active_window(&windows).ok_or_else(|| FelisError::UnexpectedError {
        message: "Couldn't find active focused window".to_string(),
    })?;

    Ok(window.id)
}

/// # Errors
///
/// Will return Err if Kitty terminal related operations fail
pub async fn open_in_helix(
    path: &AbsolutePath,
    kitty_tab_id: Option<WindowId>,
    kitty: &KittyTerminal,
    steel: bool,
) -> Result<()> {
    let windows = kitty.ls().await?;
    let kitty_window = if let Some(id) = kitty_tab_id {
        find_window_by_id(&windows, id).ok_or_else(|| FelisError::UnexpectedError {
            message: format!("Couldn't find window with id {id}"),
        })?
    } else {
        find_workspace(&windows, path)?
    };

    // Once we have the kitty window where helix is running, we can use it to potentially  shorten
    // the absolute path to a relative path from helix's working directory. This can speed up
    // "typing" the path into helix. We also remove any new lines from the path (this can happen
    // when the path is copied from a small terminal window)
    let rel_path = {
        let p = path
            .as_ref()
            .strip_prefix(window_cwd(kitty_window))?
            .to_string_lossy()
            .replace('\n', "");
        p.trim().to_owned()
    };

    if steel {
        std::fs::write("/tmp/felis-open.txt", rel_path.as_bytes())?;

        kitty.focus_window(Matcher::Id(kitty_window.id)).await?;
        // Go to normal mode by hitting ESC
        kitty.send_text(Matcher::Id(kitty_window.id), r"\E").await?;
        // Go to command mode and run felis-open command
        kitty
            .send_text(Matcher::Id(kitty_window.id), r":felis-open\r")
            .await?;
    } else {
        // Send open command directly to helix

        kitty.focus_window(Matcher::Id(kitty_window.id)).await?;
        // Go to normal mode by hitting ESC
        kitty.send_text(Matcher::Id(kitty_window.id), r"\E").await?;
        // Go to command mode
        kitty.send_text(Matcher::Id(kitty_window.id), r":").await?;
        // Paste the path first to avoid autocompletion triggering on the path segment after each
        // character
        kitty
            .send_text(Matcher::Id(kitty_window.id), &rel_path)
            .await?;
        // Jump at the beginning of the command line, type open, then hit ENTER
        kitty
            .send_text(Matcher::Id(kitty_window.id), r"\x01open \r")
            .await?;
    }

    Ok(())
}

fn find_window_by_id(windows: &OsWindows, window_id: WindowId) -> Option<&Window> {
    windows.0.iter().find_map(|os_window| {
        os_window
            .tabs
            .iter()
            .find_map(|tab| tab.windows.iter().find(|window| window.id == window_id))
    })
}

// TODO: this function should be in a different module probably
#[must_use]
pub fn focused_active_window(windows: &OsWindows) -> Option<&model::Window> {
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

// TODO: this function should be in a different module probably
#[must_use]
pub fn window_cwd(window: &Window) -> &Path {
    window.foreground_processes[0].cwd.as_path()
}

fn find_workspace<'a>(windows: &'a OsWindows, path: &AbsolutePath) -> Result<&'a Window> {
    let workspace_window = windows.0.iter().find_map(|os_window| {
        os_window.tabs.iter().find_map(|tab| {
            tab.windows.iter().find(|w| {
                w.foreground_processes
                    .iter()
                    .any(|process| is_helix_bin(process) && is_in_workspace(process, path))
            })
        })
    });

    workspace_window.ok_or_else(|| FelisError::UnexpectedError {
        message: format!("Couldn't find workspace for file {:?}", path.as_ref()),
    })
}

fn is_in_workspace(process: &model::Process, path: &AbsolutePath) -> bool {
    path.as_ref()
        .parent()
        .map_or(false, |p| p.starts_with(process.cwd.as_path()))
}

fn is_helix_bin(process: &model::Process) -> bool {
    process.cmdline.iter().any(|c| c.ends_with("bin/hx"))
}

#[cfg(test)]
mod test {

    use std::{
        os::unix::process::ExitStatusExt,
        process::{ExitStatus, Output},
    };

    use kitty_remote_bindings::{
        command::{options::Matcher, FocusWindow, Ls, SendText},
        model::WindowId,
    };
    use mockall::predicate::*;
    use pretty_assertions::assert_eq;

    use crate::{
        command::{get_active_focused_window, open_in_helix},
        fs::AbsolutePath,
        kitty_terminal::{test_fixture, KittyTerminal, MockExecutor},
    };

    fn expect_ls_success(executor: &mut MockExecutor) {
        executor
            .expect_ls()
            .times(1)
            .with(eq(Ls::new().to("DummySocket".to_string())))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
    }

    fn expect_send_text_success(executor: &mut MockExecutor, text: &str, window_id: WindowId) {
        let cmd = SendText::new(text.to_string())
            .matcher(Matcher::Id(window_id))
            .to("DummySocket".to_string());
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
        let cmd = FocusWindow::new()
            .matcher(Matcher::Id(window_id))
            .to("DummySocket".to_string());
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
    async fn test_get_active_focused_window() {
        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);

        let response = get_active_focused_window(&KittyTerminal::mock(executor))
            .await
            .unwrap();
        assert_eq!(response, WindowId(2));
    }

    #[tokio::test]
    async fn test_open_in_helix_with_kitty_tab_id() {
        let path = "/path/to/felis/src/lib.rs";

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_focus_window_succes(&mut executor, WindowId(1));
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(&mut executor, r":", WindowId(1));
        expect_send_text_success(&mut executor, r"src/lib.rs", WindowId(1));
        expect_send_text_success(&mut executor, r"\x01open \r", WindowId(1));

        open_in_helix(
            &AbsolutePath::try_from(path).unwrap(),
            Some(WindowId(1)),
            &KittyTerminal::mock(executor),
            false,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_open_in_helix_turns_absolute_path_to_relative() {
        let path = "/path/to/felis/src/lib.rs";

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_focus_window_succes(&mut executor, WindowId(1));
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(&mut executor, r":", WindowId(1));
        expect_send_text_success(&mut executor, r"src/lib.rs", WindowId(1));
        expect_send_text_success(&mut executor, r"\x01open \r", WindowId(1));

        open_in_helix(
            &AbsolutePath::try_from(path).unwrap(),
            Some(WindowId(1)),
            &KittyTerminal::mock(executor),
            false,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id() {
        let path = "/path/to/felis/src/lib.rs";

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(&mut executor, r":", WindowId(1));
        expect_send_text_success(&mut executor, r"src/lib.rs", WindowId(1));
        expect_send_text_success(&mut executor, r"\x01open \r", WindowId(1));
        expect_focus_window_succes(&mut executor, WindowId(1));

        open_in_helix(
            &AbsolutePath::try_from(path).unwrap(),
            None,
            &KittyTerminal::mock(executor),
            false,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id_command_resolves_relative_path() {
        let path = "/path/to/felis/src/lib.rs";

        let mut executor = MockExecutor::new();
        expect_ls_success(&mut executor);
        expect_send_text_success(&mut executor, r"\E", WindowId(1));
        expect_send_text_success(&mut executor, r":", WindowId(1));
        expect_send_text_success(&mut executor, r"src/lib.rs", WindowId(1));
        expect_send_text_success(&mut executor, r"\x01open \r", WindowId(1));
        expect_focus_window_succes(&mut executor, WindowId(1));

        open_in_helix(
            &AbsolutePath::try_from(path).unwrap(),
            None,
            &KittyTerminal::mock(executor),
            false,
        )
        .await
        .unwrap();
    }
}
