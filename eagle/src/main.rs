mod runtime;
mod sinks;
mod sources;

use std::{collections::HashMap, sync::Arc, time::Instant};

use futures::{channel::mpsc, SinkExt, StreamExt};

use eagle_core::{
    EagleEndpoint, EagleEvent, Event, MetricEvent, MetricFilter, MetricSink, Origin, Source,
};
use runtime::{
    sink::{spawn_sink, SinkConfig, SinkState},
    source::{spawn_source, SourceState},
};
use sinks::Console;
use sources::Disks;
use tokio::task::JoinHandle;
use uuid::Uuid;

enum Recv {
    Available(EagleEvent),
    Disconnected,
}

struct MainReceiver {
    inner: mpsc::UnboundedReceiver<EagleEvent>,
}

impl MainReceiver {
    pub async fn recv(&mut self) -> Recv {
        if let Some(event) = self.inner.next().await {
            return Recv::Available(event);
        }

        Recv::Disconnected
    }
}

#[derive(Clone)]
struct SinkClient {
    inner: mpsc::UnboundedSender<MetricEvent>,
}

impl SinkClient {
    pub async fn send(&self, event: MetricEvent) -> bool {
        self.inner.clone().send(event).await.is_ok()
    }
}

struct SinkEndpoint {
    inner: mpsc::UnboundedReceiver<MetricEvent>,
}

fn new_main_bus() -> (EagleEndpoint, MainReceiver) {
    let (sender, recv) = mpsc::unbounded::<EagleEvent>();

    (EagleEndpoint::new(sender), MainReceiver { inner: recv })
}

struct MainProcess {
    endpoint: EagleEndpoint,
    handle: JoinHandle<()>,
}

impl MainProcess {
    pub async fn wait_until_complete(self) {
        let _ = self.handle.await;
    }
}

pub struct SourceId(Uuid);

pub struct Configuration {
    sources: Vec<SourceState>,
    sinks: Vec<SinkState>,
    endpoint: EagleEndpoint,
    receiver: MainReceiver,
}

impl Configuration {
    pub fn new() -> Self {
        let (endpoint, receiver) = new_main_bus();
        Self {
            sources: Default::default(),
            sinks: Default::default(),
            endpoint,
            receiver,
        }
    }

    pub fn register_source<S>(&mut self, origin: Origin, source: S)
    where
        S: Source + Send + 'static,
    {
        let state = spawn_source(origin, self.endpoint.clone(), source);

        self.sources.push(state);
    }

    pub fn register_sink<S>(&mut self, config: SinkConfig, sink: S)
    where
        S: MetricSink + Send + 'static,
    {
        let state = spawn_sink(config, sink);

        self.sinks.push(state);
    }
}

fn start_main_process(conf: Configuration) -> MainProcess {
    let mut main_recv = conf.receiver;
    let endpoint = conf.endpoint;
    let _sources = conf.sources;
    let mut sinks = conf.sinks;
    let handle = tokio::spawn(async move {
        let mut deads = Vec::new();
        let mut errored = true;

        while let Recv::Available(event) = main_recv.recv().await {
            match event.event {
                Event::Metric(metric) => {
                    let metric = Arc::new(metric);

                    for sink in sinks.iter_mut() {
                        if sink.is_handled(event.origin.as_ref(), metric.as_ref()) {
                            if !sink.send_metric(event.origin.clone(), metric.clone()).await {
                                // TODO - Means that a sink did and we might consider restarting or
                                // shutdown the damn application completly.
                                tracing::error!(
                                    target = "main-process",
                                    "Sink {} died",
                                    sink.name()
                                );

                                deads.push(sink.id());
                            }
                        }
                    }

                    // We remove all dead sinks from the sink roaster.
                    if !deads.is_empty() {
                        sinks.retain(|s| !deads.contains(&s.id()));
                        deads.clear();
                    }
                }

                Event::Tick => {}

                Event::Shutdown => {
                    for sink in sinks {
                        sink.shutdown().await;
                    }

                    errored = false;
                    break;
                }
            }
        }

        if errored {
            tracing::error!(target = "main-process", "Main process exited unexpectedly");
        }
    });

    MainProcess { endpoint, handle }
}

#[tokio::main]
async fn main() {
    let mut conf = Configuration::new();

    conf.register_sink(SinkConfig::new("console"), Console);
    conf.register_source(
        Origin::new("host"),
        Disks::new(vec!["/dev/nvme0n1".to_string()]),
    );

    let process = start_main_process(conf);

    process.wait_until_complete().await;
}
