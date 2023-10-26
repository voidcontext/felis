use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::{command, util::WritePayloadExt, Result};

use super::command_listener::CommandListener;
use crate::util::ReadPayloadExt;

pub async fn listen<R, C>(command_listener: C)
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
{
    loop {
        if let Ok(command::SHUTDOWN) = handle_connection(&command_listener).await {
            break;
        }
    }
}

async fn handle_connection<R, C>(command_listener: &C) -> Result<u8>
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
{
    match command_listener.accept().await {
        Ok(mut reader_writer) => {
            let cmd = reader_writer
                .read_u8()
                .await
                .expect("Couldn't read command from reader");

            match cmd {
                command::SHUTDOWN => (),
                command::ECHO => {
                    let msg = reader_writer.read_payload().await?;

                    reader_writer.write_payload(&msg).await?;
                }
                _ => todo!(),
            };

            Ok(cmd)
        }
        Err(_) => todo!(),
    }
}

#[cfg(test)]
mod test {

    use pretty_assertions::assert_eq;

    use crate::{command, server::command_listener::stubs::CommandListenerStub};

    use super::listen;

    #[allow(clippy::assertions_on_constants)]
    #[tokio::test]
    async fn test_shutdown_command() {
        let cl = CommandListenerStub::new(vec![command::SHUTDOWN]);

        listen(cl).await;

        assert!(true);
    }

    #[allow(clippy::assertions_on_constants)]
    #[tokio::test]
    async fn test_echo_command() {
        let message = b"test message";
        let mut test_packet = vec![];
        test_packet.extend_from_slice(
            &TryInto::<u16>::try_into(message.len())
                .unwrap()
                .to_be_bytes(),
        );
        test_packet.extend_from_slice(message);

        let cl = CommandListenerStub::new({
            let mut cmd = vec![command::ECHO];
            cmd.extend_from_slice(&test_packet);
            cmd.push(command::SHUTDOWN);
            cmd
        });

        let written = cl.written();

        listen(cl).await;

        let written = &*written.lock().unwrap();
        assert_eq!(written, &test_packet);
    }
}
