mod runtime;
mod sources;

use std::{cell::RefCell, collections::HashMap, sync::Arc, time::Instant};

use futures::{channel::mpsc, SinkExt, StreamExt};

use eagle_core::{EagleEndpoint, EagleEvent, Event, MetricEvent, MetricFilter, MetricSink, Source};
use runtime::sink::{spawn_sink, SinkConfig, SinkState};
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

struct SinkProcess {
    filter: MetricFilter,
    client: SinkClient,
    last_time: Option<Instant>,
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

#[derive(Default)]
pub struct Configuration {
    sources: HashMap<Uuid, Box<dyn Source>>,
    sinks: Vec<SinkState>,
}

impl Configuration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_source<S>(&mut self, name: impl AsRef<str>, source: S) -> SourceId
    where
        S: Source + Send,
    {
        let id = Uuid::new_v4();

        SourceId(id)
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
    let (endpoint, mut main_recv) = new_main_bus();
    let mut sinks = conf.sinks;
    let handle = tokio::spawn(async move {
        let mut deads = Vec::new();
        while let Recv::Available(event) = main_recv.recv().await {
            match event.event {
                Event::Metric(metric) => {
                    let metric = Arc::new(metric);

                    for sink in sinks.iter_mut() {
                        if sink.is_handled(metric.as_ref()) {
                            if !sink.send_metric(metric.clone()).await {
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

                    // We remove all dead sinks from the sink roster.
                    if !deads.is_empty() {
                        sinks.retain(|s| !deads.contains(&s.id()));
                        deads.clear();
                    }
                }

                Event::Tick => {}
            }
        }
    });

    MainProcess { endpoint, handle }
}

#[tokio::main]
async fn main() {
    let mut conf = Configuration::new();
    let process = start_main_process(conf);

    process.wait_until_complete().await;
}
