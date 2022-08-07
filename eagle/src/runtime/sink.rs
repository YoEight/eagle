use std::{time::Instant, sync::Arc};

use eagle_core::{Metric, MetricFilter, MetricSink, MetricEvent};
use futures::{channel::mpsc, SinkExt, StreamExt};
use tokio::task::JoinHandle;
use uuid::Uuid;

enum Msg {
    Metric(Arc<Metric>),
    Tick,
}

#[derive(Clone)]
pub struct SinkClient {
    inner: mpsc::Sender<Msg>,
}

impl SinkClient {
    pub async fn shutdown(&self) {}

    pub async fn send_metric(&self, metric: Arc<Metric>) -> bool {
        self.inner.clone().send(Msg::Metric(metric)).await.is_ok()
    }

    pub async fn send_tick(&self) -> bool {
        self.inner.clone().send(Msg::Tick).await.is_ok()
    }
}

pub struct SinkState {
    client: SinkClient,
    config: SinkConfig,
    last_time: Option<Instant>,
    handle: JoinHandle<()>,
}

impl SinkState {
    pub fn id(&self) -> Uuid {
        self.config.id
    }

    pub fn is_handled(&self, metric: &Metric) -> bool {
        self.config.filter.is_handled(metric)
    }

    pub async fn send_metric(&mut self, metric: Arc<Metric>) -> bool {
        let result = self.client.send_metric(metric).await;
        self.last_time = Some(Instant::now());

        result
    }

    pub fn name(&self) -> &str {
        self.config.name.as_str()
    }
}

pub struct SinkConfig {
    id: Uuid,
    name: String,
    filter: MetricFilter,
}

impl SinkConfig {
    pub fn new(name: impl AsRef<str>) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            name: format!("sink-{}:{}", name.as_ref(), id),
            filter: MetricFilter::no_filter(),
        }
    }

    pub fn set_filter(mut self, filter: MetricFilter) -> Self {
        Self {
            filter,
            ..self
        }
    }
}

pub fn spawn_sink<S>(config: SinkConfig, mut sink: S) -> SinkState
where
    S: MetricSink + Send + 'static,
{
    let (inner, mut recv) = mpsc::channel(500);
    let client = SinkClient { inner };
    let name = config.name.to_string();
    let handle = tokio::spawn(async move {
        tracing::info!(target = name.as_str(), "Sink started");

        while let Some(msg) = recv.next().await {
            match msg {
                Msg::Metric(m) => {
                    sink.process(MetricEvent::Metric(m.clone())).await;
                    tracing::debug!(target = name.as_str(), "Metric '{}' processed", m.name);
                }
                Msg::Tick => {
                    tracing::debug!(target = name.as_str(), "Ticking completed");
                }
            }
        }

        tracing::warn!(target = name.as_str(), "Sink exited");
    });

    SinkState {
        config,
        client,
        handle,
        last_time: None,
    }
}
