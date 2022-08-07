use std::time::Instant;

use eagle_core::{Metric, MetricFilter, MetricSink};
use futures::{channel::mpsc, SinkExt, StreamExt};
use tokio::task::JoinHandle;

enum Msg {
    Metric(Metric),
    Tick,
}

#[derive(Clone)]
pub struct SinkClient {
    inner: mpsc::Sender<Msg>,
}

impl SinkClient {
    pub async fn shutdown(&self) {}

    pub async fn send_metric(&self, metric: Metric) -> bool {
        self.inner.clone().send(Msg::Metric(metric)).await.is_ok()
    }

    pub async fn send_tick(&self) -> bool {
        self.inner.clone().send(Msg::Tick).await.is_ok()
    }
}

pub struct SinkState {
    filter: MetricFilter,
    client: SinkClient,
    last_time: Option<Instant>,
    handle: JoinHandle<()>,
}

pub fn spawn_sink<S>(name: impl AsRef<str>, sink: S) -> SinkState
where
    S: MetricSink + Send,
{
    let filter = sink.filter();
    let (inner, mut recv) = mpsc::channel(500);
    let client = SinkClient { inner };
    let name = format!("sink-{}", name.as_ref());

    let handle = tokio::spawn(async move {
        while let Some(msg) = recv.next().await {
            match msg {
                Msg::Metric(_) => {}
                Msg::Tick => {}
            }
        }

        tracing::warn!(target = name.as_str(), "Sink exited");
    });

    SinkState {
        filter,
        client,
        handle,
        last_time: None,
    }
}
