use std::process::{ExitStatus, Output};

use crate::{FelisError, Result};
use felis_protocol::{WireRead, WireReadError, WireReadResult, WireWrite, WireWriteResult};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    process::Command,
};

use async_trait::async_trait;

#[derive(Clone, Copy, PartialEq)]
pub enum Flag {
    NoOp = 0b00,
    DryRun = 0b01,
}

// TODO: derive these
#[async_trait]
impl<R: AsyncRead + Unpin + Send> WireRead<R> for Flag {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>> {
        let byte = reader.read_u8().await?;

        match byte {
            0b00 => Ok(Box::new(Flag::NoOp)),
            0b01 => Ok(Box::new(Flag::DryRun)),
            f => Err(WireReadError::UnexpectedError {
                message: format!("Invalid flag: {f}"),
            }),
        }
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WireWrite<W> for Flag {
    async fn write(&self, writer: &mut W) -> WireWriteResult {
        (*self as u8).write(writer).await?;
        Ok(())
    }
}

#[async_trait]
pub trait Executor {
    async fn execute(&self, cmd: &mut Command, flag: &Flag) -> Result<Output>;

    async fn execute_all(&self, cmds: &mut [Command], flag: &Flag) -> Result<Output> {
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
    async fn execute(&self, cmd: &mut Command, flag: &Flag) -> Result<Output> {
        if Flag::DryRun == *flag {
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
    async fn execute(&self, cmd: &mut Command, _flag: &Flag) -> Result<Output> {
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
    async fn execute(&self, cmd: &mut Command, flag: &Flag) -> Result<Output> {
        match flag {
            Flag::DryRun => DryRun.execute(cmd, flag).await,
            Flag::NoOp => TokioRuntime.execute(cmd, flag).await,
        }
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use tokio::process::Command;

    use crate::server::executor::Flag;

    use super::{DryRun, Executor};

    #[tokio::test]
    async fn test_dry_run_executor_should_return_command_line() {
        let result = DryRun
            .execute(Command::new("echo").arg("Hello World!"), &Flag::NoOp)
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
        let result = DryRun.execute_all(&mut cmd, &Flag::NoOp).await.unwrap();

        let stdout = String::from_utf8(result.stdout).unwrap();
        assert_eq!(
            stdout.as_str(),
            r#""echo" "Hello World!"
"ls" "foobar"
"#
        );
    }
}
