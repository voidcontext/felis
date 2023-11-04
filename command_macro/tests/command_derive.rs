use std::vec;

use pretty_assertions::assert_eq;

use felis_command::*;
use felis_command_macro::*;

#[derive(Debug, PartialEq)]
#[command(14)]
struct Simple;

#[tokio::test]
async fn test_to_bytes_simple() {
    let mut buf = Vec::new();

    Simple.write(&mut buf).await.unwrap();

    assert_eq!(buf, vec![14u8]);
}

#[tokio::test]
async fn test_from_bytes_simple() {
    let buf: [u8; 0] = [];
    assert_eq!(*Simple::read(&mut buf.as_slice()).await.unwrap(), Simple);
}

#[derive(Debug, PartialEq)]
#[command(15)]
struct CommandWithStringAndNumber {
    payload: String,
    number: u16,
    wrapped: Wrapped,
}

#[derive(Debug, PartialEq, ReadWire, WriteWire)]
struct Wrapped {
    number: usize,
}

#[tokio::test]
async fn test_to_bytes_command_with_string_and_number() {
    let payload = b"test message";
    let mut bytes = vec![15u8];
    bytes.extend_from_slice(&payload.len().to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&1200u16.to_be_bytes());
    bytes.extend_from_slice(&5000usize.to_be_bytes());

    let mut buf = Vec::new();

    CommandWithStringAndNumber {
        payload: String::from("test message"),
        number: 1200,
        wrapped: Wrapped { number: 5000 },
    }
    .write(&mut buf)
    .await
    .unwrap();

    assert_eq!(buf, bytes);
}

#[tokio::test]
async fn test_from_bytes_command_with_string_and_number() {
    let payload = b"test message";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&payload.len().to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&1200u16.to_be_bytes());
    bytes.extend_from_slice(&5000usize.to_be_bytes());
    assert_eq!(
        *CommandWithStringAndNumber::read(&mut bytes.as_slice())
            .await
            .unwrap(),
        CommandWithStringAndNumber {
            payload: String::from("test message"),
            number: 1200,
            wrapped: Wrapped { number: 5000 }
        }
    );
}
