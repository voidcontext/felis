#![allow(clippy::missing_errors_doc)]
use std::io;
use std::process::Output;

use crate::Result;
use async_trait::async_trait;
use kitty_remote_bindings::model::OsWindows;
use kitty_remote_bindings::{CommandOutput, Ls, Matcher, MatcherExt, SendText};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub(crate) trait Executor {
    async fn ls(&self, ls: &Ls) -> io::Result<Output>;
    async fn send_text(&self, send_text: &SendText) -> io::Result<Output>;
}

struct TokioExecutor;
#[async_trait]
impl Executor for TokioExecutor {
    async fn ls(&self, ls: &Ls) -> io::Result<Output> {
        tokio::process::Command::from(Into::<std::process::Command>::into(ls))
            .output()
            .await
    }
    async fn send_text(&self, send_text: &SendText) -> io::Result<Output> {
        tokio::process::Command::from(Into::<std::process::Command>::into(send_text))
            .output()
            .await
    }
}

pub struct KittyTerminal {
    executor: Box<dyn Executor + Send + Sync + 'static>,
}

impl KittyTerminal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            executor: Box::new(TokioExecutor),
        }
    }

    #[cfg(test)]
    pub(crate) fn mock(mock_executor: MockExecutor) -> Self {
        Self {
            executor: Box::new(mock_executor),
        }
    }

    pub async fn ls(&self) -> Result<OsWindows> {
        let output = self.executor.ls(&Ls::new()).await?;
        let result = Ls::result(&output)?;

        Ok(result)
    }

    pub async fn send_text(&self, matcher: Matcher, text: &str) -> Result<()> {
        let mut cmd = SendText::new(text.to_string());
        cmd.matcher(matcher);
        let output = self.executor.send_text(&cmd).await?;

        SendText::result(&output)?;

        Ok(())
    }
}

impl Default for KittyTerminal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::process::{ExitStatus, Output};

    use kitty_remote_bindings::{model::WindowId, Ls, Matcher, MatcherExt, SendText};
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;

    use super::{test_fixture, KittyTerminal, MockExecutor};

    #[tokio::test]
    async fn test_ls_should_execute_the_ls_remote_command() {
        let mut executor = MockExecutor::new();

        executor
            .expect_ls()
            .with(eq(Ls::new()))
            .times(1)
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::default(),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });

        let terminal = KittyTerminal {
            executor: Box::new(executor),
        };

        let result = terminal.ls().await.expect("ls() returned an error");

        assert_eq!(result, *test_fixture::LS_OUTPUT);
    }

    #[tokio::test]
    async fn test_send_text_should_execute_the_send_text_remote_command() {
        let mut executor = MockExecutor::new();

        let matcher = Matcher::Id(WindowId(9));

        let mut cmd = SendText::new("text message".to_string());
        cmd.matcher(matcher.clone());

        executor
            .expect_send_text()
            .with(eq(cmd))
            .times(1)
            .returning(|_| {
                Ok(Output {
                    status: ExitStatus::default(),
                    stdout: test_fixture::LS_OUTPUT_JSON.as_bytes().to_vec(),
                    stderr: Vec::new(),
                })
            });

        let terminal = KittyTerminal {
            executor: Box::new(executor),
        };

        terminal
            .send_text(matcher, "text message")
            .await
            .expect("ls() returned an error");
    }
}

#[cfg(test)]
pub mod test_fixture {

    use lazy_static::lazy_static;

    use kitty_remote_bindings::model::{
        OsWindow, OsWindowId, OsWindows, Process, Tab, TabId, Window, WindowId,
    };

    lazy_static! {
    pub static ref LS_OUTPUT: OsWindows = OsWindows(
        vec![
            OsWindow {
                id: OsWindowId(1u32),
                is_active: true,
                is_focused: true,
                tabs: vec![
                    Tab {
                        id: TabId(1u32),
                        is_active: true,
                        is_focused: true,
                        windows: vec![
                            Window {
                                id: WindowId(1u32),
                                is_active: false,
                                is_focused: false,
                                foreground_processes: vec![
                                Process {
                                    cmdline: vec![
                                      "/nix/store/6z1v4fzjw416c38j4013y9wam07q5zbs-rust-default-1.73.0/libexec/rust-analyzer-proc-macro-srv".to_string()
                                    ],
                                    cwd: "/path/to/felis".to_string(),
                                    pid: 40339
                                },
                                Process {
                                  cmdline: vec![
                                    "/nix/store/0g95h72qqdxlig31n6ahcz1ch1jsg9q4-rust-analyzer-unwrapped-2023-05-15/bin/rust-analyzer".to_string()
                                  ],
                                  cwd: "/path/to/felis".to_string(),
                                  pid: 38646
                                },
                                Process {
                                  cmdline: vec![
                                    "/etc/profiles/per-user/gaborpihaj/bin/hx".to_string()
                                  ],
                                  cwd: "/path/to/felis".to_string(),
                                  pid: 38411
                              }],
                            },
                            Window {
                                id: WindowId(2u32),
                                is_active: true,
                                is_focused: true,
                                foreground_processes: vec![
                                    Process {
                                        pid: 49915,
                                        cwd: "/path/to/felis".to_string(),
                                        cmdline: vec![
                                            "kitten".to_string(),
                                            "@".to_string(),
                                            "ls".to_string(),
                                        ],
                                    },
                                ]
                            },
                            Window {
                                id: WindowId(3u32),
                                is_active: false,
                                is_focused: false,
                                foreground_processes: vec![
                                    Process {
                                        pid: 983,
                                        cwd: "/path/to/felis".to_string(),
                                        cmdline: vec![
                                            "-zsh".to_string(),
                                        ],
                                    },
                                ],
                            }
                        ],
                    }
                ],
            }
        ]
    );
    }

    pub static LS_OUTPUT_JSON: &str = r#"[
{
    "id": 1,
    "is_active": true,
    "is_focused": true,
    "last_focused": true,
    "platform_window_id": 130,
    "tabs": [
      {
        "active_window_history": [
          3,
          2,
          1
        ],
        "enabled_layouts": [
          "fat",
          "grid",
          "horizontal",
          "splits",
          "stack",
          "tall",
          "vertical"
        ],
        "id": 1,
        "is_active": true,
        "is_focused": true,
        "layout": "grid",
        "layout_opts": {},
        "layout_state": {
          "biased_cols": {},
          "biased_rows": {}
        },
        "title": "kitty @ ls",
        "windows": [
          {
            "cmdline": [
              "-zsh"
            ],
            "columns": 119,
            "cwd": "/path/to/felis",
            "env": {},
            "foreground_processes": [
              {
                "cmdline": [
                  "/nix/store/6z1v4fzjw416c38j4013y9wam07q5zbs-rust-default-1.73.0/libexec/rust-analyzer-proc-macro-srv"
                ],
                "cwd": "/path/to/felis",
                "pid": 40339
              },
              {
                "cmdline": [
                  "/nix/store/0g95h72qqdxlig31n6ahcz1ch1jsg9q4-rust-analyzer-unwrapped-2023-05-15/bin/rust-analyzer"
                ],
                "cwd": "/path/to/felis",
                "pid": 38646
              },
              {
                "cmdline": [
                  "/etc/profiles/per-user/gaborpihaj/bin/hx"
                ],
                "cwd": "/path/to/felis",
                "pid": 38411
              }
            ],
            "id": 1,
            "is_active": false,
            "is_focused": false,
            "is_self": false,
            "lines": 47,
            "pid": 863,
            "title": "hx"
          },
          {
            "cmdline": [
              "-zsh"
            ],
            "columns": 119,
            "cwd": "/path/to/felis",
            "env": {},
            "foreground_processes": [
              {
                "cmdline": [
                  "kitten",
                  "@",
                  "ls"
                ],
                "cwd": "/path/to/felis",
                "pid": 49915
              }
            ],
            "id": 2,
            "is_active": true,
            "is_focused": true,
            "is_self": true,
            "lines": 23,
            "pid": 972,
            "title": "kitty @ ls"
          },
          {
            "cmdline": [
              "-zsh"
            ],
            "columns": 119,
            "cwd": "/path/to/felis",
            "env": {},
            "foreground_processes": [
              {
                "cmdline": [
                  "-zsh"
                ],
                "cwd": "/path/to/felis",
                "pid": 983
              }
            ],
            "id": 3,
            "is_active": false,
            "is_focused": false,
            "is_self": false,
            "lines": 24,
            "pid": 983,
            "title": "/path/to/felis"
          }
        ]
      }
    ],
    "wm_class": "kitty",
    "wm_name": "kitty"
  }
]"#;
}
