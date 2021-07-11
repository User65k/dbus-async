use hex::encode;
use std::io::Error as IoError;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream};

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("Could not list available mechanisms")]
    NoMechanism,
    #[error("Could not authenticate")]
    NoAuthentication,
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("Authentication error: {0}")]
    NegotiateUnixFdError(String),
    #[error("IO Error: {0}")]
    IoError(#[from] IoError),
}

const NEW_LINE: &str = "\r\n";

pub(super) struct Handshake<T>(BufStream<T>);

impl<T> Handshake<T>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    async fn read_line(&mut self) -> Result<String, IoError> {
        let mut line = String::new();
        self.0.read_line(&mut line).await?;
        if let Some(line) = line.strip_suffix(NEW_LINE) {
            Ok(line.to_owned())
        } else {
            Ok(line)
        }
    }

    async fn write_line(&mut self, line: &str) -> Result<(), IoError> {
        self.0.write_all(line.as_bytes()).await?;
        self.0.write_all(NEW_LINE.as_bytes()).await?;
        self.0.flush().await?;
        Ok(())
    }

    async fn list_available_mechanisms(&mut self) -> Result<Vec<String>, HandshakeError> {
        self.write_line("AUTH").await?;

        let resp = self.read_line().await?;
        if let Some(mechanisms) = resp.strip_prefix("REJECTED ") {
            let mut result = Vec::new();
            for mechanism in mechanisms.split(' ') {
                result.push(mechanism.to_owned());
            }

            if result.is_empty() {
                Err(HandshakeError::NoMechanism)
            } else {
                Ok(result)
            }
        } else {
            Err(HandshakeError::NoMechanism)
        }
    }

    async fn negotiate_unix_fd(&mut self) -> Result<(), HandshakeError> {
        self.write_line("NEGOTIATE_UNIX_FD").await?;

        let resp = self.read_line().await?;
        // Check if the authentication is successful.
        if resp == "AGREE_UNIX_FD" {
            Ok(())
        } else {
            Err(HandshakeError::NegotiateUnixFdError(resp))
        }
    }

    async fn auth_external(&mut self) -> Result<(), HandshakeError> {
        // Get the UID of the process
        let uid = unsafe { libc::getuid() };
        // Encode the UID in a hex string.
        let hex = encode(uid.to_string());
        // Authenticate to the DBus daemon.
        let cmd = format!("AUTH EXTERNAL {}", hex);
        self.write_line(cmd.as_str()).await?;
        // Read the response of the socket.
        let resp = self.read_line().await?;
        // Check if the authentication is successful.
        if resp.starts_with("OK ") {
            Ok(())
        } else {
            Err(HandshakeError::AuthenticationError(resp))
        }
    }

    async fn auth_anonymous(&mut self) -> Result<(), HandshakeError> {
        self.write_line("AUTH ANONYMOUS 646275732d6173796e63")
            .await?;
        // Read the response of the socket.
        let resp = self.read_line().await?;
        // Check if the authentication is successful.
        if resp.starts_with("OK ") {
            Ok(())
        } else {
            Err(HandshakeError::AuthenticationError(resp))
        }
    }

    async fn authenticate(&mut self) -> Result<(), HandshakeError> {
        for mechanism in self.list_available_mechanisms().await? {
            match mechanism.as_str() {
                "EXTERNAL" => match self.auth_external().await {
                    Ok(_) => return Ok(()),
                    Err(e) => error!("Could not authenticate (EXTERNAL): {}", e),
                },
                "ANONYMOUS" => match self.auth_anonymous().await {
                    Ok(_) => return Ok(()),
                    Err(e) => error!("Could not authenticate (ANONYMOUS): {}", e),
                },
                x => error!("Authentication is not supported: {}", x),
            }
        }

        Err(HandshakeError::NoAuthentication)
    }

    async fn begin(mut self) -> Result<(), IoError> {
        // Authentication was successful.
        self.write_line("BEGIN").await
    }

    async fn new(stream: T) -> Result<Handshake<T>, IoError> {
        let mut buf_stream = BufStream::new(stream);
        // Connect to the Unix Domain Stream.
        // let mut stream = TokioUnixStream::connect(path).await?;
        // Write a zero to the socket.
        let zero: [u8; 1] = [0; 1];
        buf_stream.write_all(&zero[..]).await?;
        Ok(Handshake(buf_stream))
    }

    /// Connect to the Unix Domain Stream socket.
    pub(super) async fn handshake(
        stream: &mut T,
        negotiate_unix_fd: bool,
    ) -> Result<(), HandshakeError> {
        let mut handshake = Handshake::new(stream).await?;

        handshake.authenticate().await?;

        if negotiate_unix_fd {
            handshake.negotiate_unix_fd().await?;
        }

        handshake.begin().await?;
        Ok(())
    }
}
