use eagle_core::{EagleClient, MetricBuilder, Source};
use heim::units::ratio::ratio;
use tokio::time::Duration;

pub struct Load;

#[async_trait::async_trait]
impl Source for Load {
    async fn produce(&mut self, client: EagleClient) {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            match heim::cpu::os::unix::loadavg().await {
                Err(e) => {
                    tracing::error!(
                        target = client.origin().instance_id(),
                        "Failed to load load average info: {}",
                        e
                    );
                }

                Ok(loadavg) => {
                    if let Err(e) = client
                        .send_metrics(vec![
                            MetricBuilder::gauge("host", "load1", loadavg.0.get::<ratio>() as f64)
                                .build(),
                            MetricBuilder::gauge("host", "load5", loadavg.1.get::<ratio>() as f64)
                                .build(),
                            MetricBuilder::gauge("host", "load15", loadavg.2.get::<ratio>() as f64)
                                .build(),
                        ])
                        .await
                    {
                        tracing::error!(
                            target = client.origin().instance_id(),
                            "Error when sending metrics: {}",
                            e
                        );

                        break;
                    }
                }
            }

            clock.tick().await;
        }
    }
}
