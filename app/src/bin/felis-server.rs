use felis::server::executor;
use felis::server::{self, UnixSocket};
use felis::Command;
use felis::Result;
use felis_protocol::WireWrite;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::exfiltrator::SignalOnly;
use signal_hook::iterator::SignalsInfo;
use tokio::net::UnixStream;

async fn signal_handler(mut signals: SignalsInfo, socket_path: &str) {
    for signal in &mut signals {
        match signal {
            SIGTERM | SIGINT | SIGQUIT => {
                let mut socket = UnixStream::connect(socket_path).await.unwrap();
                Command::Shutdown.write(&mut socket).await.unwrap();
                break;
            }
            _ => unreachable!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket_path = "/tmp/felis.sock";
    let socket = UnixSocket::new(socket_path)?;

    // Set up signal handler
    let signals = SignalsInfo::<SignalOnly>::new([SIGTERM, SIGINT, SIGQUIT])?;
    let handle = signals.handle();
    let signals_task = tokio::spawn(signal_handler(signals, socket_path));

    // run the server
    server::listen(&socket, &executor::Configurable).await;

    // clean up
    handle.close();
    signals_task.abort();
    tokio::fs::remove_file(socket_path).await?;

    Ok(())
}
