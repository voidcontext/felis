use std::path::Path;

use crate::Result;
use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{UnixListener, UnixStream},
};

#[async_trait]
pub trait CommandListener<R: AsyncRead + AsyncWrite> {
    async fn accept(&self) -> std::io::Result<R>;
}

pub struct UnixSocket {
    listener: UnixListener,
}

impl UnixSocket {
    /// # Errors
    ///
    /// Will return error if binding the listener to the socket fails
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let listener = UnixListener::bind(path)?;

        Ok(Self { listener })
    }
}

#[async_trait]
impl CommandListener<UnixStream> for UnixSocket {
    async fn accept(&self) -> std::io::Result<UnixStream> {
        let (stream, _) = self.listener.accept().await?;
        Ok(stream)
    }
}

#[cfg(test)]
pub mod stubs {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use test_utils::ReaderWriterStub;

    use super::CommandListener;

    pub struct CommandListenerStub {
        reader_writer: ReaderWriterStub,
    }

    impl CommandListenerStub {
        #[must_use]
        pub fn new(commands: Vec<u8>) -> Self {
            CommandListenerStub {
                reader_writer: ReaderWriterStub::new(commands),
            }
        }

        #[must_use]
        pub fn written(&self) -> Arc<Mutex<Vec<u8>>> {
            self.reader_writer.written()
        }
    }

    #[async_trait]
    impl CommandListener<ReaderWriterStub> for CommandListenerStub {
        async fn accept(&self) -> std::io::Result<ReaderWriterStub> {
            Ok(self.reader_writer.clone())
        }
    }
}
