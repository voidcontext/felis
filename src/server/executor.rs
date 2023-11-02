use std::process::{ExitStatus, Output};

use crate::{FelisError, Result};
use tokio::process::Command;

use async_trait::async_trait;

pub enum Flag {
    DryRun = 0b0001,
}

#[async_trait]
pub trait Executor {
    async fn execute(&self, cmd: &mut Command, flag: &Option<Flag>) -> Result<Output>;

    async fn execute_all(&self, cmds: &mut [Command], flag: &Option<Flag>) -> Result<Output> {
        let mut aggregated_ouput = Output {
            status: ExitStatus::default(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };

        for cmd in cmds {
            let out = self.execute(cmd, flag).await?;
            aggregated_ouput.status = out.status;
            aggregated_ouput.stdout.extend_from_slice(&out.stdout);
            aggregated_ouput.stderr.extend_from_slice(&out.stderr);

            if !aggregated_ouput.status.success() {
                break;
            }
        }

        Ok(aggregated_ouput)
    }
}

pub struct TokioRuntime;

#[async_trait]
impl Executor for TokioRuntime {
    async fn execute(&self, cmd: &mut Command, flag: &Option<Flag>) -> Result<Output> {
        if let Some(Flag::DryRun) = flag {
            return Err(FelisError::UnexpectedError {
                message: String::from("The TokioRuntime executor doesn't support the DryRun Flag"),
            });
        }

        let output = cmd.output().await?;
        Ok(output)
    }
}

pub struct DryRun;

#[async_trait]
impl Executor for DryRun {
    async fn execute(&self, cmd: &mut Command, _flag: &Option<Flag>) -> Result<Output> {
        Ok(Output {
            status: ExitStatus::default(),
            stdout: format!("{:?}\n", cmd.as_std()).as_bytes().to_vec(),
            stderr: Vec::new(),
        })
    }
}

pub struct Configurable;

#[async_trait]
impl Executor for Configurable {
    async fn execute(&self, cmd: &mut Command, flag: &Option<Flag>) -> Result<Output> {
        if let Some(Flag::DryRun) = flag {
            DryRun.execute(cmd, flag).await
        } else {
            TokioRuntime.execute(cmd, flag).await
        }
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use tokio::process::Command;

    use super::{DryRun, Executor};

    #[tokio::test]
    async fn test_dry_run_executor_should_return_command_line() {
        let result = DryRun
            .execute(Command::new("echo").arg("Hello World!"), &None)
            .await
            .unwrap();

        let stdout = String::from_utf8(result.stdout).unwrap();
        assert_eq!(
            stdout.as_str(),
            r#""echo" "Hello World!"
"#
        );
    }

    #[tokio::test]
    async fn test_dry_run_executor_execute_all_should_aggregate_putputs() {
        let mut echo = Command::new("echo");
        echo.arg("Hello World!");

        let mut ls = Command::new("ls");
        ls.arg("foobar");

        let mut cmd = vec![echo, ls];
        let result = DryRun.execute_all(&mut cmd, &None).await.unwrap();

        let stdout = String::from_utf8(result.stdout).unwrap();
        assert_eq!(
            stdout.as_str(),
            r#""echo" "Hello World!"
"ls" "foobar"
"#
        );
    }
}
