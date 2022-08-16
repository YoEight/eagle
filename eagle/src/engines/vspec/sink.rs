use std::{sync::Arc, time::Instant};

use eagle_core::{
    config::{SinkConfig, SinkDecl},
    EagleSink, EagleStream, Metric, MetricEvent, Origin, eagle_channel,
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
        let result = self.client.send_msg(MetricEvent::Metric {
            origin: origin.clone(),
            metric: metric.clone(),
        }).await;

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

pub fn spawn_sink(handle: &Handle, decl: SinkDecl) -> SinkState {
    let (inner, mut recv) = mpsc::channel(500);
    let id = decl.id;
    let name = decl.name.clone();
    let cloned_name = decl.name.clone();
    let config = decl.config;
    let mut sink = decl.sink;
    let (client, eagle_stream) = eagle_channel(500);
    let handle = handle.spawn(async move {
        let mut errored = true;
        tracing::info!(target = decl.name.as_str(), "Sink started");

        while let Some(msg) = recv.recv().await {
            match msg {
                Msg::Metric(o, m) => {
                    let event = MetricEvent::Metric {
                        origin: o.clone(),
                        metric: m.clone(),
                    };

                    if publisher.send(event).is_err() {
                        tracing::error!(
                            target = name.as_str(),
                            "Sink '{}' unexpectedly shutdown",
                            name
                        );

                        break;
                    }

                    tracing::debug!(target = name.as_str(), "Metric '{}' processed", m.name);
                }

                Msg::Tick => {
                    tracing::debug!(target = name.as_str(), "Ticking completed");
                }

                Msg::Shutdown => {
                    tracing::info!(
                        target = name.as_str(),
                        "Sink '{}' shutdown successfully",
                        name
                    );
                    errored = false;
                    break;
                }
            }
        }

        if errored {
            tracing::error!(target = name.as_str(), "Sink exited unexpectedly");
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
