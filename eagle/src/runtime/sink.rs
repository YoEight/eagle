use std::time::Instant;

use eagle_core::Metric;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct SinkClient {}

impl SinkClient {
    pub async fn shutdown(&self) {}

    pub async fn send_metric(&self, metric: Metric) -> bool {
        false
    }

    pub async fn send_tick(&self) -> bool {
        false
    }
}

pub struct SinkState {
    last_time: Instant,
    handle: JoinHandle<()>,
}
