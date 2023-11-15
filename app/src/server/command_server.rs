use std::path::{Path, PathBuf};

use felis_protocol::{WireRead, WireWrite};
use kitty_remote_bindings::{
    model::{self, OsWindows, WindowId},
    Matcher,
};
use tokio::io::{AsyncRead, AsyncWrite};

use super::command_listener::CommandListener;
use crate::{kitty_terminal::KittyTerminal, Command, FelisError, Response, Result};

pub async fn listen<R, C>(command_listener: &C, kitty: &KittyTerminal)
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
{
    loop {
        match handle_connection(command_listener, kitty).await {
            Ok(Command::Shutdown) => break,
            Ok(_) => (),
            Err(e) => println!("An error happened: {e:?}"),
        }
    }
}

async fn handle_connection<R, C>(command_listener: &C, kitty: &KittyTerminal) -> Result<Command>
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
{
    match command_listener.accept().await {
        Ok(mut reader_writer) => {
            let cmd = *Command::read(&mut reader_writer).await?;
            match &cmd {
                Command::Shutdown => (),
                Command::Echo(msg) => {
                    Response::Message((*msg).clone())
                        .write(&mut reader_writer)
                        .await?;
                }
                Command::OpenInHelix { path, kitty_tab_id } => {
                    let kitty_window_id = if let Some(id) = kitty_tab_id {
                        WindowId(*id)
                    } else {
                        let windows = kitty.ls().await?;
                        find_workspace(&windows, path)?
                    };

                    kitty.send_text(Matcher::Id(kitty_window_id), r"\E").await?;
                    kitty
                        .send_text(
                            Matcher::Id(kitty_window_id),
                            &[format!(":open {}", path.to_string_lossy()).as_str(), r"\r"].join(""),
                        )
                        .await?;

                    Response::Ack.write(&mut reader_writer).await?;
                }
            };
            Ok(cmd)
        }
        Err(_) => todo!(),
    }
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
            path_buf.push(window.foreground_processes[0].cwd.as_path());
            path_buf.push(path);
            path_buf
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}

fn find_workspace(windows: &OsWindows, path: &Path) -> Result<WindowId> {
    let path = resolve_relative_path(windows, path);

    let workspace_window = windows.0.iter().find_map(|os_window| {
        os_window.tabs.iter().find_map(|tab| {
            tab.windows
                .iter()
                .find(|w| {
                    w.foreground_processes
                        .iter()
                        .any(|process| is_helix_bin(process) && is_in_workspace(process, &path))
                })
                .map(|w| w.id)
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
        future,
        os::unix::process::ExitStatusExt,
        path::PathBuf,
        process::{ExitStatus, Output},
    };

    use felis_protocol::{WireRead, WireWrite};
    use kitty_remote_bindings::{model::WindowId, Ls, Matcher, MatcherExt, SendText};
    use mockall::predicate::*;
    use pretty_assertions::assert_eq;
    use test_utils::ReaderWriterStub;

    use crate::{
        kitty_terminal::{test_fixture, KittyTerminal, MockExecutor},
        server::command_listener::MockCommandListener,
        Command, Response,
    };

    use super::listen;

    #[allow(clippy::assertions_on_constants)]
    #[tokio::test]
    async fn test_shutdown_command() {
        let mut buf = Vec::new();
        Command::Shutdown.write(&mut buf).await.unwrap();

        let reader_writer_stub = ReaderWriterStub::new(buf);
        let mut cl = MockCommandListener::new();
        cl.expect_accept()
            .returning(move || Box::pin(future::ready(Ok(reader_writer_stub.clone()))));

        listen(&cl, &KittyTerminal::mock(MockExecutor::new())).await;

        assert!(true);
    }

    #[tokio::test]
    async fn test_echo_command() {
        let mut buf = Vec::new();
        Command::Echo("test message".to_string())
            .write(&mut buf)
            .await
            .unwrap();
        Command::Shutdown.write(&mut buf).await.unwrap();

        let reader_writer_stub = ReaderWriterStub::new(buf);
        let mut cl = MockCommandListener::new();
        cl.expect_accept().returning({
            let rws = reader_writer_stub.clone();
            move || Box::pin(future::ready(Ok(rws.clone())))
        });

        listen(&cl, &KittyTerminal::mock(MockExecutor::new())).await;

        let written_bytes = {
            let written = reader_writer_stub.written();
            let written_bytes = written.lock().unwrap();
            (*written_bytes).clone()
        }; // drop the non async aware mutex guard at the end of the scope

        let response = Response::read(&mut written_bytes.as_slice())
            .await
            .expect("Couldn't read response");
        assert_eq!(*response, Response::Message("test message".to_string()));
    }

    #[tokio::test]
    async fn test_open_in_helix_with_kitty_tab_id_command() {
        let path = "/path/to/felis/src/lib.rs";

        let mut buf = Vec::new();
        Command::OpenInHelix {
            kitty_tab_id: Some(1),
            path: PathBuf::from(path),
        }
        .write(&mut buf)
        .await
        .unwrap();
        Command::Shutdown.write(&mut buf).await.unwrap();

        let reader_writer_stub = ReaderWriterStub::new(buf);
        let mut cl = MockCommandListener::new();
        cl.expect_accept().returning({
            let rws = reader_writer_stub.clone();
            move || Box::pin(future::ready(Ok(rws.clone())))
        });

        let mut executor = MockExecutor::new();

        let mut cmd = SendText::new(r"\E".to_string());
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
        let mut cmd = SendText::new(format!(r":open {path}\r"));
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
        listen(&cl, &KittyTerminal::mock(executor)).await;

        let written_bytes = {
            let written = reader_writer_stub.written();
            let written_bytes = written.lock().unwrap();
            (*written_bytes).clone()
        }; // drop the non async aware mutex guard at the end of the scope

        let response = Response::read(&mut written_bytes.as_slice())
            .await
            .expect("Couldn't read response");
        assert_eq!(*response, Response::Ack);
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id_command() {
        let path = "/path/to/felis/src/lib.rs";

        let mut buf = Vec::new();
        Command::OpenInHelix {
            kitty_tab_id: None,
            path: PathBuf::from(path),
        }
        .write(&mut buf)
        .await
        .unwrap();
        Command::Shutdown.write(&mut buf).await.unwrap();

        let mut executor = MockExecutor::new();
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

        let mut cmd = SendText::new(r"\E".to_string());
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
        let mut cmd = SendText::new(format!(r":open {path}\r"));
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });

        let reader_writer_stub = ReaderWriterStub::new(buf);
        let mut cl = MockCommandListener::new();
        cl.expect_accept().returning({
            let rws = reader_writer_stub.clone();
            move || Box::pin(future::ready(Ok(rws.clone())))
        });

        listen(&cl, &KittyTerminal::mock(executor)).await;

        let written_bytes = {
            let written = reader_writer_stub.written();
            let written_bytes = written.lock().unwrap();
            (*written_bytes).clone()
        }; // drop the non async aware mutex guard at the end of the scope

        let response = Response::read(&mut written_bytes.as_slice())
            .await
            .expect("Couldn't read response");
        assert_eq!(*response, Response::Ack);
    }

    #[tokio::test]
    async fn test_open_in_helix_without_kitty_tab_id_command_resolves_relative_path() {
        let path = "src/lib.rs";

        let mut buf = Vec::new();
        Command::OpenInHelix {
            kitty_tab_id: None,
            path: PathBuf::from(path),
        }
        .write(&mut buf)
        .await
        .unwrap();
        Command::Shutdown.write(&mut buf).await.unwrap();

        let mut executor = MockExecutor::new();
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

        let mut cmd = SendText::new(r"\E".to_string());
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });
        let mut cmd = SendText::new(format!(r":open {path}\r"));
        cmd.matcher(Matcher::Id(WindowId(1)));
        executor
            .expect_send_text()
            .times(1)
            .with(eq(cmd))
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });

        let reader_writer_stub = ReaderWriterStub::new(buf);
        let mut cl = MockCommandListener::new();
        cl.expect_accept().returning({
            let rws = reader_writer_stub.clone();
            move || Box::pin(future::ready(Ok(rws.clone())))
        });

        listen(&cl, &KittyTerminal::mock(executor)).await;

        let written_bytes = {
            let written = reader_writer_stub.written();
            let written_bytes = written.lock().unwrap();
            (*written_bytes).clone()
        }; // drop the non async aware mutex guard at the end of the scope

        let response = Response::read(&mut written_bytes.as_slice())
            .await
            .expect("Couldn't read response");
        assert_eq!(*response, Response::Ack);
    }
}
