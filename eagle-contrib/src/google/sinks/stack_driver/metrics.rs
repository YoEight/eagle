use eagle_core::{MetricEvent, MetricSink};

use super::types::StackDriverMetricsOptions;
use std::{collections::HashMap, time::Instant};

pub struct StackDriverMetrics {
    options: StackDriverMetricsOptions,
    clock: Instant,
    buffer: HashMap<String, ()>,
}

impl StackDriverMetrics {
    pub fn new(options: StackDriverMetricsOptions) -> Self {
        Self {
            options,
            clock: Instant::now(),
            buffer: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl MetricSink for StackDriverMetrics {
    async fn process(&mut self, event: MetricEvent) {
        match event {
            MetricEvent::Metric { metric, .. } => {}
            MetricEvent::Tick => {
                if self.clock.elapsed() < self.options.period {
                    return;
                }
            }
        }

        self.clock = Instant::now();
    }
}
