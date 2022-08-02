mod runtime;
mod sources;

use std::{cell::RefCell, collections::HashMap, sync::Arc, time::Instant};

use futures::{channel::mpsc, SinkExt, StreamExt};

use eagle_core::{EagleEndpoint, EagleEvent, Event, MetricEvent, MetricFilter, MetricSink, Source};
use runtime::sink::SinkState;
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

#[derive(Default)]
pub struct Configuration {
    sources: HashMap<Uuid, Box<dyn Source>>,
    sinks: HashMap<Uuid, RefCell<SinkState>>,
}

fn start_main_process(mut conf: Configuration) -> MainProcess {
    let (endpoint, mut main_recv) = new_main_bus();
    let mut sinks = Vec::<SinkProcess>::new();
    let handle = tokio::spawn(async move {
        while let Recv::Available(event) = main_recv.recv().await {
            match event.event {
                Event::Metric(metric) => {
                    let metric = Arc::new(metric);
                    let metric_event = MetricEvent::Metric(metric.clone());

                    for sink in sinks.iter_mut() {
                        if sink.filter.is_handled(metric.as_ref()) {
                            if !sink.client.send(metric_event.clone()).await {
                                // TODO - Means that a sink did and we might consider restarting or
                                // shutdown the damn application completly.
                            }

                            sink.last_time = Some(Instant::now());
                        }
                    }
                }
            }
        }
    });

    MainProcess { endpoint, handle }
}

#[tokio::main]
async fn main() {
    let process = start_main_process();

    process.wait_until_complete().await;
}
