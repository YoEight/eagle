use eagle_core::{MetricEvent, MetricSink};

pub struct Console;

#[async_trait::async_trait]
impl MetricSink for Console {
    async fn process(&mut self, event: MetricEvent) {
        match event {
            MetricEvent::Tick => {}
            MetricEvent::Metric { origin, metric } => {
                println!(
                    "Source '{source}', InstanceId '{instance_id}', Metric: {metric:?}",
                    source = origin.name,
                    instance_id = origin.instance_id,
                    metric = metric,
                );
            }
        }
    }
}
