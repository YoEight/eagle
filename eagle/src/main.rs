use std::{sync::Arc, time::Instant};

use futures::{channel::mpsc, SinkExt, StreamExt};

use eagle_core::{EagleEvent, MetricEvent, MetricFilter};

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
struct MainBus {
    inner: mpsc::UnboundedSender<EagleEvent>,
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

fn new_main_bus() -> (MainBus, MainReceiver) {
    let (sender, recv) = mpsc::unbounded::<EagleEvent>();

    (MainBus { inner: sender }, MainReceiver { inner: recv })
}

async fn run_main_loop(mut main_recv: MainReceiver) {
    let mut sinks = Vec::<SinkProcess>::new();
    let handle = tokio::spawn(async move {
        while let Recv::Available(event) = main_recv.recv().await {
            match event {
                EagleEvent::Metric(metric) => {
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

    let _ = handle.await;
}

#[tokio::main]
async fn main() {
    let (sender, recv) = new_main_bus();

    run_main_loop(recv).await;
}
