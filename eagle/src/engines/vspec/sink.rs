use std::{sync::Arc, time::Instant};

use eagle_core::{
    config::{SinkConfig, SinkDecl},
    eagle_channel, EagleSink, EagleStream, Metric, MetricEvent, Origin,
};
use tokio::{
    runtime::{Handle, Runtime},
    sync::mpsc,
    task::JoinHandle,
    time::Duration,
};
use uuid::Uuid;

pub struct SinkState {
    id: Uuid,
    name: String,
    client: EagleSink<MetricEvent>,
    config: SinkConfig,
    last_time: Option<Instant>,
    handle: JoinHandle<()>,
}

impl SinkState {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn is_handled(&self, origin: &Origin, metric: &Metric) -> bool {
        self.config.filter.is_handled(origin, metric)
    }

    // TODO - Consider not blocking in case on the queue is full.
    pub async fn send_metric(&mut self, origin: Arc<Origin>, metric: Arc<Metric>) -> bool {
        let result = self.client.send_msg(MetricEvent { origin, metric }).await;

        self.last_time = Some(Instant::now());

        result
    }

    pub async fn shutdown(self) {
        self.client.shutdown().await;

        match tokio::time::timeout(Duration::from_secs(10), self.handle).await {
            Ok(outcome) => {
                if let Err(e) = outcome {
                    tracing::error!(
                        target = "main-process",
                        "Sink '{}' ended unexpectedly: {}",
                        self.name,
                        e
                    );
                }
            }
            Err(_) => {
                tracing::error!(
                    target = "main-process",
                    "Sink '{}' timeout at shutting down in a timely manner",
                    self.name
                );
            }
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

pub fn spawn_sink(handle: &Handle, mut decl: SinkDecl) -> SinkState {
    let id = decl.id;
    let cloned_name = decl.name.clone();
    let config = decl.config;
    let (client, eagle_stream) = eagle_channel(500);

    let client_cloned = client.clone();
    handle.spawn(async move {
        let mut clock = tokio::time::interval(Duration::from_millis(30));

        loop {
            clock.tick().await;
            if !client_cloned.send_tick().await {
                break;
            }
        }
    });

    let handle = handle.spawn(async move {
        tracing::info!(target = decl.name.as_str(), "Sink started");
        if let Err(e) = decl.sink.process(eagle_stream).await {
            tracing::error!(
                target = decl.name.as_str(),
                "Sink exited with an unexpected error: {}",
                e
            );
        } else {
            tracing::info!(target = decl.name.as_str(), "Sink exited");
        }
    });

    SinkState {
        client,
        handle,
        last_time: None,
        id,
        name: cloned_name,
        config,
    }
}
