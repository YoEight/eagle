use eagle_core::{EagleStream, MetricEvent, MetricSink, Recv};

pub struct Console;

#[async_trait::async_trait]
impl MetricSink for Console {
    async fn process(&mut self, mut stream: EagleStream<MetricEvent>) {
        while let Recv::Available(event) = stream.recv().await {
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
}
