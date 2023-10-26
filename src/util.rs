use crate::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[async_trait]
pub trait ReadPayloadExt: AsyncRead {
    async fn read_payload(&mut self) -> Result<Vec<u8>>;
}

#[async_trait]
impl<A: AsyncRead + std::marker::Unpin + std::marker::Send> ReadPayloadExt for A {
    async fn read_payload(&mut self) -> Result<Vec<u8>> {
        let msg_len = self.read_u16().await?;
        let mut msg = vec![0; (msg_len) as usize];
        self.read_exact(&mut msg).await?;
        Ok(msg)
    }
}

#[async_trait]
pub trait WritePayloadExt: AsyncWrite {
    async fn write_payload(&mut self, msg: &[u8]) -> Result<()>;
}

#[async_trait]
impl<A: AsyncWrite + std::marker::Unpin + std::marker::Send> WritePayloadExt for A {
    async fn write_payload(&mut self, msg: &[u8]) -> Result<()> {
        self.write_u16(msg.len().try_into()?).await?;
        self.write_all(msg).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use test_utils::ReaderWriterStub;

    use crate::util::{ReadPayloadExt, WritePayloadExt};

    #[tokio::test]
    async fn test_read_payload_can_read_the_payload() {
        let msg = b"test message! \xF0\x9F\xA6\x80";
        let mut to_read: Vec<u8> = TryInto::<u16>::try_into(msg.len())
            .unwrap()
            .to_be_bytes()
            .to_vec();
        to_read.extend_from_slice(msg);

        let mut reader = ReaderWriterStub::new(to_read);
        let payload = reader.read_payload().await.unwrap();

        assert_eq!(String::from_utf8(payload).unwrap(), "test message! 🦀");
    }

    #[tokio::test]
    async fn test_write_payload_can_write_the_payload() {
        let msg = b"test message! \xF0\x9F\xA6\x80";

        let mut writer = ReaderWriterStub::new(Vec::new());
        writer.write_payload(msg).await.unwrap();

        let written = writer.written();
        let written = written.lock().unwrap();

        let mut expected: Vec<u8> = TryInto::<u16>::try_into(msg.len())
            .unwrap()
            .to_be_bytes()
            .to_vec();
        expected.extend_from_slice(msg);
        assert_eq!(*written, expected);
    }
}
