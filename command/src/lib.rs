use std::io;
use std::string::FromUtf8Error;

use async_trait::async_trait;
pub use std::pin::Pin;
use thiserror::Error;
pub use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Error, Debug)]
pub enum WireFormatReadError {
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("FromUtf8 error")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
}

#[derive(Error, Debug)]
pub enum WireFormatWriteError {
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
}

pub type ReadResult<A> = Result<A, WireFormatReadError>;
pub type WriteResult = Result<(), WireFormatWriteError>;

#[async_trait]
pub trait ReadWire<R: AsyncRead> {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>>;
}

#[async_trait]
pub trait WriteWire<W: AsyncWrite> {
    async fn write(&self, writer: &mut W) -> WriteResult;
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WriteWire<W> for u8 {
    async fn write(&self, writer: &mut W) -> WriteResult {
        writer.write_u8(*self).await?;
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> ReadWire<R> for u8 {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>> {
        let n = reader.read_u8().await?;
        Ok(Box::new(n))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WriteWire<W> for u16 {
    async fn write(&self, writer: &mut W) -> WriteResult {
        writer.write_u16(*self).await?;
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> ReadWire<R> for u16 {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>> {
        let n = reader.read_u16().await?;

        Ok(Box::new(n))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WriteWire<W> for usize {
    async fn write(&self, writer: &mut W) -> WriteResult {
        writer.write_all(&self.to_be_bytes()).await?;
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> ReadWire<R> for usize {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>> {
        let usize_byte_len = (usize::BITS / 8) as usize;
        let mut buf = vec![0; usize_byte_len];
        reader.read_exact(&mut buf).await?;

        // TODO: get rid of this unwrap
        Ok(Box::new(usize::from_be_bytes(buf.try_into().unwrap())))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WriteWire<W> for String {
    async fn write(&self, writer: &mut W) -> WriteResult {
        let bytes = self.as_bytes();

        bytes.len().write(writer).await?;
        writer.write_all(bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> ReadWire<R> for String {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>> {
        let len = usize::read(reader).await?;

        let mut buf = vec![0; *len];
        reader.read_exact(&mut buf).await?;

        let string = String::from_utf8(buf)?;

        Ok(Box::new(string))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send, T: WriteWire<W> + Sync> WriteWire<W> for Vec<T> {
    async fn write(&self, writer: &mut W) -> WriteResult {
        self.len().write(writer).await?;
        for item in self {
            item.write(writer).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send, T: ReadWire<R> + Send> ReadWire<R> for Vec<T> {
    async fn read(reader: &mut R) -> ReadResult<Box<Self>> {
        let len = usize::read(reader).await?;

        let mut result = Vec::new();
        for _ in 0..*len {
            result.push(*T::read(reader).await?);
        }

        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use crate::{ReadWire, WriteWire};

    #[tokio::test]
    async fn test_string_wire_format_to_bytes() {
        let string = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
        let mut bytes = string.as_bytes().len().to_be_bytes().to_vec();
        bytes.extend_from_slice(string.as_bytes());

        let mut buf: Vec<u8> = vec![];

        String::from(string).write(&mut buf).await.unwrap();

        assert_eq!(buf, bytes);
    }

    #[tokio::test]
    async fn test_string_wire_format_from_bytes() {
        let string = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
        let mut bytes = string.as_bytes().len().to_be_bytes().to_vec();
        bytes.extend_from_slice(string.as_bytes());

        assert_eq!(
            *String::read(&mut bytes.as_slice()).await.unwrap(),
            String::from(string)
        );
    }
}
