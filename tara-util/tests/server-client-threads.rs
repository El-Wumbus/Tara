use std::time::Duration;

use async_trait::async_trait;
use tara_util::ipc::*;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt, EnvFilter, Layer};

#[cfg(test)]
#[ctor::ctor]
fn init() {
    // Setup logging
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse("")
        .unwrap();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();
}

#[derive(Debug, Clone)]
struct ActionReceiver;

#[async_trait]
impl ActionMessageReceiver for ActionReceiver {
    async fn perform(&self, action: ActionMessage) -> ResponseMessage {
        match action {
            ActionMessage::NoOp => ResponseMessage::ActionCompleted,
            ActionMessage::EndTransmission => unreachable!(),
            _ => unimplemented!(),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn server_client_ipc_with_threads() {
    // If this struct had actual data in it you could `Arc` it and do something like this with
    // it.
    //
    // ```rust
    // let receiver = Arc::new(ActionReceiver);
    //
    // tokio::spawn({
    //     let receiver = receiver.clone();
    //     async move {
    //         start_server(receiver.as_ref()).await.unwrap();
    //     }
    // });
    // ```
    // Instead, we can just `&ActionReceiver` because it does nothing.

    tokio::spawn({
        async move {
            start_server(&ActionReceiver).await.unwrap();
        }
    });
    // Wait for the server to start up in the background
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Start client
    let client = Client::new().await.unwrap();
    for _ in 0..100 {
        let actions = &[ActionMessage::NoOp, ActionMessage::NoOp, ActionMessage::NoOp];
        let response = client.send_actions(actions).await.unwrap();
        assert_eq!(
            response,
            vec![
                ResponseMessage::ActionCompleted,
                ResponseMessage::ActionCompleted,
                ResponseMessage::ActionCompleted,
            ]
        );
    }
    client.close().await.unwrap();
}
