use std::process::Output;

use tokio::process::Command;

use async_trait::async_trait;

#[async_trait]
pub trait Executor<Out> {
    async fn execute(cmd: &mut Command) -> std::io::Result<Out>;
}

pub struct TokioRuntime;

#[async_trait]
impl Executor<Output> for TokioRuntime {
    async fn execute(cmd: &mut Command) -> std::io::Result<Output> {
        cmd.output().await
    }
}

pub struct DryRun;

#[async_trait]
impl Executor<String> for DryRun {
    async fn execute(cmd: &mut Command) -> std::io::Result<String> {
        Ok(format!("{:?}", cmd.as_std()))
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use tokio::process::Command;

    use super::{DryRun, Executor};

    #[tokio::test]
    async fn test_dry_run_executor_should_return_command_line() {
        let result = DryRun::execute(Command::new("echo").arg("Hello World!"))
            .await
            .unwrap();
        assert_eq!(result.as_str(), r#""echo" "Hello World!""#);
    }
}
