use eagle_core::{EagleMsg, EagleStream, MetricEvent, MetricSink, Recv};

pub struct Console;

#[async_trait::async_trait]
impl MetricSink for Console {
    async fn process(&mut self, mut stream: EagleStream<MetricEvent>) -> eyre::Result<()> {
        while let Recv::Available(msg) = stream.recv().await {
            match msg {
                EagleMsg::Tick => {}
                EagleMsg::Shutdown => {
                    break;
                }
                EagleMsg::Msg(event) => {
                    println!(
                        "Source '{source}', InstanceId '{instance_id}', Metric: {metric:?}",
                        source = event.origin.name,
                        instance_id = event.origin.instance_id,
                        metric = event.metric,
                    );
                }
            }
        }

        Ok(())
    }
}
