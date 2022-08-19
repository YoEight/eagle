use std::sync::Arc;

use eagle_core::{config::Configuration, EagleEndpoint, EagleEvent, Event};
use tokio::{runtime::Handle, sync::mpsc, task::JoinHandle};

use self::{sink::spawn_sink, source::spawn_source};

mod sink;
mod source;

pub struct VSpec {
    pub join: JoinHandle<()>,
    pub endpoint: EagleEndpoint,
}

enum Recv {
    Available(EagleEvent),
    Disconnected,
}

struct MainReceiver {
    inner: mpsc::UnboundedReceiver<EagleEvent>,
}

impl MainReceiver {
    pub async fn recv(&mut self) -> Recv {
        if let Some(event) = self.inner.recv().await {
            return Recv::Available(event);
        }

        Recv::Disconnected
    }
}

fn new_main_bus() -> (EagleEndpoint, MainReceiver) {
    let (sender, recv) = mpsc::unbounded_channel::<EagleEvent>();

    (EagleEndpoint::new(sender), MainReceiver { inner: recv })
}

impl VSpec {
    pub fn start(conf: Configuration) -> Self {
        VSpec::start_with_handle(&tokio::runtime::Handle::current(), conf)
    }

    pub fn start_with_handle(handle: &Handle, conf: Configuration) -> Self {
        let (endpoint, mut main_recv) = new_main_bus();

        let mut sinks = conf
            .sinks
            .into_iter()
            .map(|decl| spawn_sink(handle, decl))
            .collect::<Vec<_>>();

        let _sources = conf
            .sources
            .into_iter()
            .map(|decl| spawn_source(handle, decl, endpoint.clone()))
            .collect::<Vec<_>>();

        let join = handle.spawn(async move {
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

        VSpec { join, endpoint }
    }

    pub async fn wait_until_complete(self) -> bool {
        self.join.await.is_ok()
    }
}
