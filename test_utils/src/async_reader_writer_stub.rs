use std::sync::{Arc, Mutex};

use tokio::io::{AsyncRead, AsyncWrite};

pub struct ReaderWriterStub {
    to_read: Arc<Mutex<Vec<u8>>>,
    written: Arc<Mutex<Vec<u8>>>,
}

impl ReaderWriterStub {
    pub fn new(to_read: Vec<u8>) -> Self {
        Self {
            to_read: Arc::new(Mutex::new(to_read)),
            written: Default::default(),
        }
    }

    pub fn written(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.written)
    }
}

impl Clone for ReaderWriterStub {
    fn clone(&self) -> Self {
        ReaderWriterStub {
            to_read: Arc::clone(&self.to_read),
            written: Arc::clone(&self.written),
        }
    }
}

impl AsyncRead for ReaderWriterStub {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut to_read = self.to_read.lock().expect("Couldn't lock mutex (to_read)");
        if to_read.is_empty() {
            std::task::Poll::Ready(Ok(()))
        } else {
            let remaining = buf.remaining();
            buf.put_slice(&to_read[..remaining]);
            *to_read = to_read[remaining..].to_vec();
            std::task::Poll::Ready(Ok(()))
        }
    }
}

impl AsyncWrite for ReaderWriterStub {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut written = self.written.lock().expect("Couldn't lock mutex (written)");
        written.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
