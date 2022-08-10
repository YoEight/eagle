mod runtime;
mod sinks;
mod sources;

use std::sync::Arc;

use futures::{channel::mpsc, StreamExt};

use eagle_core::{
    config::{Configuration, SinkConfig, SourceConfig},
    EagleEndpoint, EagleEvent, Event,
};
use runtime::{sink::spawn_sink, source::spawn_source};
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

fn start_main_process(conf: Configuration) -> MainProcess {
    let (endpoint, mut main_recv) = new_main_bus();

    let mut sinks = conf.sinks.into_iter().map(spawn_sink).collect::<Vec<_>>();
    let _sources = conf
        .sources
        .into_iter()
        .map(|decl| spawn_source(decl, endpoint.clone()))
        .collect::<Vec<_>>();

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
    let conf = Configuration::default()
        .register_sink("console", SinkConfig::default(), Console)
        .register_source(
            "host",
            SourceConfig::default(),
            Disks::new(vec!["nvme0n1".to_string()]),
        );

    let process = start_main_process(conf);

    process.wait_until_complete().await;
}
