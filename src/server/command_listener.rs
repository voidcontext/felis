use std::io::Read;

use async_trait::async_trait;

#[async_trait]
pub trait CommandListener<R: Read> {
    async fn accept(&self) -> std::io::Result<R>;
}
