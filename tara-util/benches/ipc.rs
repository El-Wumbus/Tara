/// Look, these aren't meant to be scientfic or super accurate.
use std::{mem::size_of, time::Duration};

use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, Criterion};
use tara_util::ipc::{self, ActionMessage, ActionMessageReceiver, Client, ResponseMessage};

#[derive(Debug, Clone)]
struct ActionReceiver;

#[async_trait]
impl ActionMessageReceiver for ActionReceiver {
    async fn perform(&self, _action: ActionMessage) -> ResponseMessage { ResponseMessage::ActionCompleted }
}

fn benchmark_ipc_multithread(c: &mut Criterion) {
    const ACTIONS: &[ActionMessage] = &[ActionMessage::NoOp];
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let gu = rt.enter();
    rt.spawn({
        async move {
            ipc::start_server(&ActionReceiver).await.unwrap();
        }
    });

    rt.block_on(tokio::time::sleep(Duration::from_millis(500)));
    let mut group = c.benchmark_group("throughput");
    group.sample_size(10_000);
    // This throughput combines the client's message and the server's response.
    group.throughput(criterion::Throughput::Bytes(
        (size_of::<ActionMessage>() + size_of::<ResponseMessage>()) as u64,
    ));

    let client = rt.block_on(async move { Client::new().await.unwrap() });
    drop(gu);

    group.bench_function("benchmark_ipc_multithread", |b| {
        let client = client.clone();
        b.to_async(&rt).iter(|| client.send_actions(ACTIONS));
    });
    group.finish();
}

criterion_group!(benches, benchmark_ipc_multithread);
criterion_main!(benches);
