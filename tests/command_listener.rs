use felis::server::{CommandListener, UnixSocket};
use pretty_assertions::assert_eq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_unix_socket_listener() {
    let dir = tempfile::tempdir().expect("Couldn't create tempdir");

    let mut socket = dir.path().to_path_buf();
    socket.push("felis.sock");

    let listener = UnixSocket::new(&socket).unwrap();
    tokio::spawn(async move {
        let mut reader_writer = listener
            .accept()
            .await
            .expect("Couldn't accept incoming connection");

        let len = reader_writer.read_u16().await.expect("Couldn't read") as usize;

        let mut message = vec![0; len];
        reader_writer
            .read_exact(&mut message)
            .await
            .expect("Couldn't read message");

        assert_eq!(
            String::from_utf8(message[..len].to_vec()).unwrap(),
            "test message"
        );

        let response = b"test response";
        reader_writer
            .write_u16(response.len().try_into().unwrap())
            .await
            .unwrap();
        reader_writer.write_all(response).await.unwrap();
    });

    let mut test_socket = tokio::net::UnixStream::connect(socket)
        .await
        .expect("Couldn't open test socket");

    let message = b"test message";

    test_socket
        .write_u16(message.len().try_into().unwrap())
        .await
        .expect("Couldn't write to socket");

    test_socket
        .write_all(message)
        .await
        .expect("Couldn't write to socket");

    let len = test_socket.read_u16().await.expect("Couldn't read") as usize;

    let mut response = vec![0; len];
    test_socket
        .read_exact(&mut response)
        .await
        .expect("Couldn't read message");

    assert_eq!(
        String::from_utf8(response[..len].to_vec()).unwrap(),
        "test response"
    );
}
