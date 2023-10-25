use std::io::Read;

use crate::command;

use super::command_listener::CommandListener;

async fn listen<R, C>(command_listener: C)
where
    R: Read,
    C: CommandListener<R>,
{
    loop {
        match command_listener.accept().await {
            Ok(mut reader) => {
                let mut cmd_buf = Vec::new();

                reader
                    .read_to_end(&mut cmd_buf)
                    .expect("Couldn't read command from reader");

                match cmd_buf.first() {
                    Some(&command::SHUTDOWN) => break,
                    _ => todo!(),
                }
            }
            Err(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;

    use crate::{command, server::command_listener::CommandListener};

    use super::listen;

    struct CommandListenerMock {
        commands: Vec<&'static [u8]>,
    }

    #[async_trait]
    impl CommandListener<&'static [u8]> for CommandListenerMock {
        async fn accept(&self) -> std::io::Result<&'static [u8]> {
            Ok(self.commands[0])
        }
    }

    fn create_listener(commands: Vec<&'static [u8]>) -> CommandListenerMock {
        CommandListenerMock { commands }
    }

    #[allow(clippy::assertions_on_constants)]
    #[tokio::test]
    async fn test_shutdown_command() {
        let cl = create_listener(vec![&[command::SHUTDOWN]]);

        listen(cl).await;

        assert!(true);
    }
}
