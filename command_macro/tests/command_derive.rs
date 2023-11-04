use pretty_assertions::assert_eq;

use felis_command::*;
use felis_command_macro::*;

#[derive(Debug, PartialEq, ReadWire, WriteWire)]
enum Command {
    NoOp,
    Unnamed(String, usize),
    Named { number: u16, wrapped: Wrapped },
}

#[derive(Debug, PartialEq, ReadWire, WriteWire)]
struct Wrapped {
    number: usize,
    message: String,
}

#[tokio::test]
async fn test_enum_unit_write() {
    let mut buf = Vec::new();

    Command::NoOp.write(&mut buf).await.unwrap();

    assert_eq!(buf, 0usize.to_be_bytes().to_vec());
}

#[tokio::test]
async fn test_enum_unit_read() {
    let buf = 0usize.to_be_bytes();
    assert_eq!(
        *Command::read(&mut buf.as_slice()).await.unwrap(),
        Command::NoOp
    );
}

#[tokio::test]
async fn test_enum_unnamed_write() {
    let mut buf = Vec::new();

    Command::NoOp.write(&mut buf).await.unwrap();

    assert_eq!(buf, 0usize.to_be_bytes().to_vec());
}

#[tokio::test]
async fn test_enum_unnamed_read() {
    let msg = "lorem ipsum";
    let number = 45332usize;
    let msg_bytes = msg.as_bytes();
    let mut buf = 1usize.to_be_bytes().to_vec();
    buf.extend_from_slice(msg_bytes.len().to_be_bytes().as_slice());
    buf.extend_from_slice(msg_bytes);
    buf.extend_from_slice(number.to_be_bytes().as_slice());
    assert_eq!(
        *Command::read(&mut buf.as_slice()).await.unwrap(),
        Command::Unnamed(msg.to_string(), number)
    );
}

#[tokio::test]
async fn test_struct_write() {
    let message = "test message".to_string();
    let message_bytes = message.as_bytes();
    let mut bytes = 5000usize.to_be_bytes().to_vec();
    bytes.extend_from_slice(message_bytes.len().to_be_bytes().as_slice());
    bytes.extend_from_slice(message_bytes);

    let mut buf = Vec::new();

    Wrapped {
        number: 5000,
        message,
    }
    .write(&mut buf)
    .await
    .unwrap();

    assert_eq!(buf, bytes);
}

#[tokio::test]
async fn test_struct_read() {
    let message = "test message".to_string();
    let message_bytes = message.as_bytes();
    let mut bytes = 5000usize.to_be_bytes().to_vec();
    bytes.extend_from_slice(message_bytes.len().to_be_bytes().as_slice());
    bytes.extend_from_slice(message_bytes);

    assert_eq!(
        *Wrapped::read(&mut bytes.as_slice()).await.unwrap(),
        Wrapped {
            number: 5000,
            message
        },
    );
}

#[tokio::test]
async fn test_enum_named_write() {
    let message = "test message".to_string();
    let message_bytes = message.as_bytes();
    let mut bytes = 2usize.to_be_bytes().to_vec();
    bytes.extend_from_slice(&1200u16.to_be_bytes());
    bytes.extend_from_slice(&5000usize.to_be_bytes());
    bytes.extend_from_slice(message_bytes.len().to_be_bytes().as_slice());
    bytes.extend_from_slice(message_bytes);

    let mut buf = Vec::new();

    Command::Named {
        number: 1200,
        wrapped: Wrapped {
            number: 5000,
            message,
        },
    }
    .write(&mut buf)
    .await
    .unwrap();

    assert_eq!(buf, bytes);
}

#[tokio::test]
async fn test_enum_named_read() {
    let message = "test message".to_string();
    let message_bytes = message.as_bytes();
    let mut bytes = 2usize.to_be_bytes().to_vec();
    bytes.extend_from_slice(&1200u16.to_be_bytes());
    bytes.extend_from_slice(&5000usize.to_be_bytes());
    bytes.extend_from_slice(message_bytes.len().to_be_bytes().as_slice());
    bytes.extend_from_slice(message_bytes);
    assert_eq!(
        *Command::read(&mut bytes.as_slice()).await.unwrap(),
        Command::Named {
            number: 1200,
            wrapped: Wrapped {
                number: 5000,
                message
            },
        }
    );
}
