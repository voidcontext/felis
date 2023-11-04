use felis_command::{Command, ReadWire, WriteWire};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    process,
};

use crate::{command, Result};

use super::{
    command_listener::CommandListener,
    executor::{Executor, Flag},
};

pub async fn listen<R, C, E>(command_listener: &C, executor: &E)
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
    E: Executor + std::marker::Sync,
{
    loop {
        match handle_connection(command_listener, executor).await {
            Ok(Some(code)) if code == command::Shutdown::code() => break,
            Ok(_) => (),
            Err(e) => println!("An error happened: {e:?}"),
        }
    }
}

async fn handle_connection<R, C, E>(command_listener: &C, executor: &E) -> Result<Option<u8>>
where
    R: AsyncRead + AsyncWrite + std::marker::Unpin + std::marker::Send,
    C: CommandListener<R>,
    E: Executor + std::marker::Sync,
{
    match command_listener.accept().await {
        Ok(mut reader_writer) => {
            let code = reader_writer.read_u8().await.ok();
            if let Some(code) = code {
                match code {
                    command::SHUTDOWN => (),
                    command::ECHO => {
                        let cmd = command::Echo::read(&mut reader_writer).await?;
                        cmd.message.write(&mut reader_writer).await?;
                    }
                    command::OPEN_IN_HELIX => {
                        let cmd = command::OpenInHelix::read(&mut reader_writer).await?;

                        let flag = if cmd.flag as u8 & Flag::DryRun as u8 == 1 {
                            Some(Flag::DryRun)
                        } else {
                            None
                        };

                        let mut commands = [
                            kitty_send_text_cmd(cmd.kitty_tab_id, r"\E"),
                            kitty_send_text_cmd(cmd.kitty_tab_id, &format!(":open {}", cmd.path)),
                            kitty_send_text_cmd(cmd.kitty_tab_id, r"\r"),
                        ];

                        let output = executor.execute_all(&mut commands, &flag).await?;
                        output.stdout.write(&mut reader_writer).await?;
                    }
                    _ => todo!(),
                };
            }

            Ok(code)
        }
        Err(_) => todo!(),
    }
}

fn kitty_send_text_cmd(tab_id: u8, text: &str) -> process::Command {
    let mut cmd = process::Command::new("kitty");

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
        test_packet.extend_from_slice(&message.len().to_be_bytes());
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
        let path = b"/path/to/some-file.txt";
        let kitty_tab = 1;
        let dry_run_executor = 0;

        let mut cmd_packet = vec![command::OPEN_IN_HELIX, dry_run_executor, kitty_tab];
        cmd_packet.extend_from_slice(&path.len().to_be_bytes());
        cmd_packet.extend_from_slice(path);
        cmd_packet.push(command::SHUTDOWN);

        let cl = CommandListenerStub::new(cmd_packet);

        let written = cl.written();

        listen(&cl, &DryRun).await;

        let expected = r#""kitty" "@" "send-text" "--match" "id:1" "\\E"
"kitty" "@" "send-text" "--match" "id:1" ":open /path/to/some-file.txt"
"kitty" "@" "send-text" "--match" "id:1" "\\r"
"#;

        let written = (*written.lock().unwrap()).clone();
        assert_eq!(String::from_utf8(written[8..].to_vec()).unwrap(), expected);
    }
}
