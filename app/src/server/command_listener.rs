use crate::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{UnixListener, UnixStream},
};

#[cfg(test)]
use mockall::automock;

#[async_trait]
#[cfg_attr(test, automock)]
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
