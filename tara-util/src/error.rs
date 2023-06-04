use thiserror::Error;
use tokio::io;

#[derive(Debug, Error)]
pub enum IpcErr {
    #[error("IO: {0}")]
    Io(io::Error),

    #[error("(de)serialization error: {0}")]
    Serialization(bincode::Error),
}

impl From<bincode::Error> for IpcErr {
    fn from(value: bincode::Error) -> Self { Self::Serialization(value) }
}

impl From<io::Error> for IpcErr {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("IO: {0}")]
    Io(io::Error),

    #[error("(de)serialization error: {0}")]
    Serialization(csv_async::Error),
}

impl From<io::Error> for LoggingError {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

impl From<csv_async::Error> for LoggingError {
    fn from(value: csv_async::Error) -> Self {
        match value.kind() {
            csv_async::ErrorKind::Io(_) => {
                if let csv_async::ErrorKind::Io(error) = value.into_kind() {
                    LoggingError::Io(error)
                } else {
                    panic!()
                }
            }
            _ => LoggingError::Serialization(value),
        }
    }
}
