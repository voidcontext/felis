use std::io;
use std::string::FromUtf8Error;

use async_trait::async_trait;
use felis_protocol_core_macro::wire_protocol_for;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Error, Debug)]
pub enum WireReadError {
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("FromUtf8 error")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
}

#[derive(Error, Debug)]
pub enum WireWriteError {
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("unexpected error: {message}")]
    UnexpectedError { message: String },
}

pub type WireReadResult<A> = Result<A, WireReadError>;
pub type WireWriteResult = Result<(), WireWriteError>;

#[async_trait]
pub trait WireRead<R: AsyncRead> {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>>;
}

#[async_trait]
pub trait WireWrite<W: AsyncWrite> {
    async fn write(&self, writer: &mut W) -> WireWriteResult;
}

#[wire_protocol_for(u8, u16, u32, u64, u128)]
#[wire_protocol_for(i8, i16, i32, i64, i128)]
#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WireWrite<W> for usize {
    async fn write(&self, writer: &mut W) -> WireWriteResult {
        writer.write_all(&self.to_be_bytes()).await?;
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> WireRead<R> for usize {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>> {
        let usize_byte_len = (usize::BITS / 8) as usize;
        let mut buf = vec![0; usize_byte_len];
        reader.read_exact(&mut buf).await?;

        // TODO: get rid of this unwrap
        Ok(Box::new(usize::from_be_bytes(buf.try_into().unwrap())))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send, T: WireWrite<W> + Send + Sync> WireWrite<W> for Option<T> {
    async fn write(&self, writer: &mut W) -> WireWriteResult {
        match self {
            Some(value) => {
                1u8.write(writer).await?;
                value.write(writer).await?;
            }
            None => 0u8.write(writer).await?,
        }

        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send, T: WireRead<R>> WireRead<R> for Option<T> {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>> {
        match *u8::read(reader).await? {
            0 => Ok(Box::new(None)),
            1 => T::read(reader).await.map(|value| Box::new(Some(*value))),
            _ => Err(WireReadError::UnexpectedError {
                message: "Not 0 or 1 for Option".to_string(),
            }),
        }
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send> WireWrite<W> for String {
    async fn write(&self, writer: &mut W) -> WireWriteResult {
        let bytes = self.as_bytes();

        bytes.len().write(writer).await?;
        writer.write_all(bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send> WireRead<R> for String {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>> {
        let len = usize::read(reader).await?;

        let mut buf = vec![0; *len];
        reader.read_exact(&mut buf).await?;

        let string = String::from_utf8(buf)?;

        Ok(Box::new(string))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send, T: WireWrite<W> + Sync> WireWrite<W> for Vec<T> {
    async fn write(&self, writer: &mut W) -> WireWriteResult {
        self.len().write(writer).await?;
        for item in self {
            item.write(writer).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send, T: WireRead<R> + Send> WireRead<R> for Vec<T> {
    async fn read(reader: &mut R) -> WireReadResult<Box<Self>> {
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

    use crate::{WireRead, WireWrite};

    #[tokio::test]
    async fn test_u8_write() {
        let mut buf: Vec<u8> = vec![];

        232u8.write(&mut buf).await.unwrap();

        assert_eq!(buf, vec![232u8]);
    }

    #[tokio::test]
    async fn test_u8_read() {
        let buf = [232u8];

        let result = *u8::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(result, 232u8);
    }

    #[tokio::test]
    async fn test_u16_write() {
        let mut buf: Vec<u8> = vec![];

        232u16.write(&mut buf).await.unwrap();

        assert_eq!(buf, 232u16.to_be_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_u16_read() {
        let buf = 232u16.to_be_bytes();

        let result = *u16::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(result, 232u16);
    }
    #[tokio::test]
    async fn test_u32_write() {
        let mut buf: Vec<u8> = vec![];

        232u32.write(&mut buf).await.unwrap();

        assert_eq!(buf, 232u32.to_be_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_u32_read() {
        let buf = 232u32.to_be_bytes();

        let result = *u32::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(result, 232u32);
    }
    #[tokio::test]
    async fn test_u64_write() {
        let mut buf: Vec<u8> = vec![];

        232u64.write(&mut buf).await.unwrap();

        assert_eq!(buf, 232u64.to_be_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_u64_read() {
        let buf = 232u64.to_be_bytes();

        let result = *u64::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(result, 232u64);
    }
    #[tokio::test]
    async fn test_usize_write() {
        let mut buf: Vec<u8> = vec![];

        232usize.write(&mut buf).await.unwrap();

        assert_eq!(buf, 232usize.to_be_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_usize_read() {
        let buf = 232usize.to_be_bytes();

        let result = *usize::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(result, 232usize);
    }

    #[tokio::test]
    async fn test_none_write() {
        let mut buf: Vec<u8> = vec![];

        Option::<String>::None.write(&mut buf).await.unwrap();

        assert_eq!(buf, vec![0u8]);
    }

    #[tokio::test]
    async fn test_none_read() {
        let buf = vec![0u8];

        let result = Option::<String>::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(*result, None);
    }

    #[tokio::test]
    async fn test_some_t_write() {
        let mut buf: Vec<u8> = vec![];

        let str = "lorem ipsum";

        Some(str.to_string()).write(&mut buf).await.unwrap();

        let mut expected = vec![1u8];
        expected.extend_from_slice(str.len().to_be_bytes().as_slice());
        expected.extend_from_slice(str.as_bytes());

        assert_eq!(buf, expected);
    }

    #[tokio::test]
    async fn test_some_t_read() {
        let str = "lorem ipsum";

        let mut buf = vec![1u8];
        buf.extend_from_slice(str.len().to_be_bytes().as_slice());
        buf.extend_from_slice(str.as_bytes());

        let result = Option::<String>::read(&mut buf.as_slice()).await.unwrap();

        assert_eq!(*result, Some(str.to_string()));
    }

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
