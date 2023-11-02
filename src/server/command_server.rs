use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    process::Command,
};

use crate::{command, util::WritePayloadExt, Result};

use super::{
    command_listener::CommandListener,
    executor::{Executor, Flag},
};
use crate::util::ReadPayloadExt;

pub async fn listen<R, C, E>(command_listener: &C, executor: &E)
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
    E: Executor + std::marker::Sync,
{
    loop {
        if let Ok(command::SHUTDOWN) = handle_connection(command_listener, executor).await {
            break;
        }
    }
}

async fn handle_connection<R, C, E>(command_listener: &C, executor: &E) -> Result<u8>
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
    E: Executor + std::marker::Sync,
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
                command::OPEN_IN_HELIX => {
                    let config_flag = reader_writer.read_u8().await?;
                    let kitty_tab = reader_writer.read_u8().await?;
                    let path = String::from_utf8(reader_writer.read_payload().await?)?;

                    let flag = if config_flag & Flag::DryRun as u8 == 1 {
                        Some(Flag::DryRun)
                    } else {
                        None
                    };

                    let mut commands = [
                        kitty_send_text_cmd(kitty_tab, r"\E"),
                        kitty_send_text_cmd(kitty_tab, &format!(":open {path}")),
                        kitty_send_text_cmd(kitty_tab, r"\r"),
                    ];

                    let output = executor.execute_all(&mut commands, &flag).await?;

                    reader_writer.write_payload(&output.stdout).await?;
                }
                _ => todo!(),
            };

            Ok(cmd)
        }
        Err(_) => todo!(),
    }
}

fn kitty_send_text_cmd(tab_id: u8, text: &str) -> Command {
    let mut cmd = Command::new("kitty");

    cmd.args([
        "@",
        "send-text",
        "--match",
        format!("id:{tab_id}").as_str(),
        text,
    ]);

    cmd
}

#[cfg(test)]
mod test {

    use pretty_assertions::assert_eq;

    use crate::{
        command,
        server::{command_listener::stubs::CommandListenerStub, executor::DryRun},
    };

    use super::listen;

    #[allow(clippy::assertions_on_constants)]
    #[tokio::test]
    async fn test_shutdown_command() {
        let cl = CommandListenerStub::new(vec![command::SHUTDOWN]);

        listen(&cl, &DryRun).await;

        assert!(true);
    }

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

        listen(&cl, &DryRun).await;

        let written = cl.written();
        let written = &*written.lock().unwrap();
        assert_eq!(written, &test_packet);
    }

    #[tokio::test]
    async fn test_open_in_helix_command() {
        let path = "/path/to/some-file.txt";
        let kitty_tab = 1;
        let dry_run_executor = 0;
        let mut cmd_packet = vec![command::OPEN_IN_HELIX, dry_run_executor, kitty_tab];
        cmd_packet.extend_from_slice(&TryInto::<u16>::try_into(path.len()).unwrap().to_be_bytes());
        cmd_packet.extend_from_slice(path.as_bytes());
        cmd_packet.push(command::SHUTDOWN);

        let cl = CommandListenerStub::new(cmd_packet);

        let written = cl.written();

        listen(&cl, &DryRun).await;

        let expected = r#""kitty" "@" "send-text" "--match" "id:1" "\\E"
"kitty" "@" "send-text" "--match" "id:1" ":open /path/to/some-file.txt"
"kitty" "@" "send-text" "--match" "id:1" "\\r"
"#;

        let written = (*written.lock().unwrap()).clone();
        assert_eq!(String::from_utf8(written[2..].to_vec()).unwrap(), expected);
    }
}
