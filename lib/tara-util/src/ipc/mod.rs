use std::{fmt::Debug, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_lite::io::BufReader;
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use serde::{Deserialize, Serialize};
use socket::SocketExt;
use tokio::{fs, sync::Mutex};
use tracing::{debug, error, info, warn};

use crate::{current_process_instance_count, error::IpcErr, paths};

pub mod socket;

/// An action reqested by the client to be performed by Tara
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionMessage {
    /// Closes the [`Client`]'s IPC connection
    EndTransmission,
    NoOp,
    GetCommandLogs {
        /// How new can logs be before they get filtered out
        upper_cutoff: Option<DateTime<Utc>>,
        /// How old can logs be before they get filtered out
        lower_cutoff: DateTime<Utc>,
    },
}

/// The server's response to a requested action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResponseMessage {
    TransmissonEnded,
    ActionCompleted,
    /// The error message is sent as a [`String`]
    ActionFailed(String),
    CommandLogs(Vec<super::logging::LoggedCommandEvent>),
}

impl<T: std::error::Error> From<T> for ResponseMessage {
    fn from(value: T) -> Self { Self::ActionFailed(value.to_string()) }
}

/// The reciever on the server that performs the actions and responds with a
/// [`ResponseMessage`].
///
/// # Example Impementaion
///
/// ```
/// # use async_trait::async_trait;
/// # use tara_util::ipc::*;
/// #[derive(Debug, Clone)]
/// struct ActionReceiver;
///
/// #[async_trait]
/// impl ActionMessageReceiver for ActionReceiver {
///     async fn perform(&self, action: ActionMessage) -> ResponseMessage {
///         match action {
///             ActionMessage::NoOp => ResponseMessage::ActionCompleted,
///             ActionMessage::EndTransmission => unreachable!(),
///             _ => unimplemented!(), // ...
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ActionMessageReceiver {
    /// Performs the requested action and finishes with a response
    async fn perform(&self, action: ActionMessage) -> ResponseMessage;
}

/// The IPC listener function. It acts as a server and the function only exits on an
/// error.
pub async fn start_server<R: ActionMessageReceiver>(action_receiver: &R) -> Result<(), IpcErr> {
    let socket_name = paths::TARA_IPC_SOCKET_FILE.as_str();

    if let Some(socket_path_parent) = PathBuf::from(socket_name).parent() && !socket_path_parent.exists() {
        fs::create_dir_all(socket_path_parent).await?;
    }

    info!("Binding to {socket_name}...");
    let listener = match LocalSocketListener::bind(socket_name) {
        Ok(l) => Ok(l),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if current_process_instance_count()? > 1 {
                error!("Only one instance of Tara can be running at once!");
                Err(e)
            } else {
                warn!("Removing existing socket: \"{socket_name}\"...");
                fs::remove_file(&socket_name).await?;
                LocalSocketListener::bind(socket_name)
            }
        }
        Err(e) => Err(e),
    }?;

    loop {
        let mut conn = match listener.accept().await {
            Ok(c) => BufReader::new(c),

            Err(e) => {
                error!("Inbound connection failed: {e}");
                continue;
            }
        };

        loop {
            let action: ActionMessage = conn.read_serde().await?;
            debug!("Server received action: {action:#?}");

            if action == ActionMessage::EndTransmission {
                conn.write_serde(ResponseMessage::TransmissonEnded).await?;
                break;
            }

            // Perform the requested actions and write the responses.
            let response = action_receiver.perform(action).await;
            conn.write_serde(response).await?;
        }
    }
}

#[derive(Debug, Clone)]
/// A [`Client`] contains an IPC connection. It Uses an [`Arc`] internally so it's cheap
/// to clone.
pub struct Client {
    connection: Arc<Mutex<BufReader<LocalSocketStream>>>,
}

impl Client {
    /// Create a new [`Client`] with an open IPC connection
    pub async fn new() -> Result<Self, IpcErr> {
        let socket_name = paths::TARA_IPC_SOCKET_FILE.as_str();
        info!("Connecting to socket: \"{socket_name}\"");
        let connection = Arc::new(Mutex::new(BufReader::new(
            LocalSocketStream::connect(socket_name).await?,
        )));
        Ok(Self { connection })
    }

    /// Send a singular action and receive a singular response.
    ///
    /// ```no_run
    /// # use tara_util::ipc::*;
    /// # tokio_test::block_on(async {
    /// # let client = Client::new().await.unwrap();
    /// let response = client
    ///     .send_action(ActionMessage::EndTransmission)
    ///     .await
    ///     .unwrap();
    /// assert_eq!(response, ResponseMessage::TransmissonEnded);
    /// # });
    /// ```
    pub async fn send_action(&self, action: ActionMessage) -> Result<ResponseMessage, IpcErr> {
        let mut connection = self.connection.lock().await;
        connection.write_serde(action).await?;
        connection.read_serde().await
    }

    /// Send multiple actions and receive multiple responses.
    ///
    /// ```no_run
    /// # use tara_util::ipc::*;
    /// # tokio_test::block_on(async {
    /// # let client = Client::new().await.unwrap();
    /// let responses = client
    ///     .send_actions(&vec![ActionMessage::NoOp; 3])
    ///     .await
    ///     .unwrap();
    /// assert_eq!(responses, vec![ResponseMessage::ActionCompleted; 2]);
    /// # });
    /// ```
    pub async fn send_actions(&self, actions: &[ActionMessage]) -> Result<Vec<ResponseMessage>, IpcErr> {
        let mut responses = Vec::with_capacity(actions.len());
        let mut connection = self.connection.lock().await;
        for action in actions {
            connection.write_serde(*action).await?;
            responses.push(connection.read_serde().await?);
        }

        debug_assert_eq!(actions.len(), responses.len());
        Ok(responses)
    }

    /// Close the [`Client`]'s connection.
    ///
    /// ```no_run
    /// # use tara_util::ipc::*;
    /// # tokio_test::block_on(async {
    /// # let client = Client::new().await.unwrap();
    /// client.close().await.unwrap();
    /// # });
    /// ```
    pub async fn close(self) -> Result<(), IpcErr> {
        let mut connection = self.connection.lock().await;
        connection.write_serde(ActionMessage::EndTransmission).await?;
        connection.read_serde::<ResponseMessage>().await.map(|_| ())
    }
}
