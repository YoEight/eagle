use std::{sync::Arc, time::Instant};

use eagle_core::{
    config::{SinkConfig, SinkDecl},
    eagle_channel, EagleSink, EagleStream, Metric, MetricEvent, Origin,
};
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
    time::Duration,
};
use uuid::Uuid;

pub struct SinkState {
    origin: Arc<Origin>,
    client: EagleSink<MetricEvent>,
    config: SinkConfig,
    last_time: Option<Instant>,
    handle: JoinHandle<()>,
}

impl SinkState {
    pub fn id(&self) -> Uuid {
        self.origin.id
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
                        self.origin.instance_id(),
                        e
                    );
                }
            }
            Err(_) => {
                tracing::error!(
                    target = "main-process",
                    "Sink '{}' timeout at shutting down in a timely manner",
                    self.origin.instance_id(),
                );
            }
        }
    }

    pub fn name(&self) -> &str {
        self.origin.instance_id()
    }
}

pub fn spawn_sink(handle: &Handle, mut decl: SinkDecl) -> SinkState {
    let sink_origin = Arc::new(decl.origin);
    let origin = sink_origin.clone();
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
        tracing::info!(target = sink_origin.instance_id(), "Sink started");
        if let Err(e) = decl.sink.process(sink_origin.clone(), eagle_stream).await {
            tracing::error!(
                target = sink_origin.instance_id(),
                "Sink exited with an unexpected error: {}",
                e
            );
        } else {
            tracing::info!(target = sink_origin.instance_id(), "Sink exited");
        }
    });

    SinkState {
        origin,
        client,
        handle,
        last_time: None,
        config,
    }
}
