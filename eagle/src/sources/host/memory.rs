use eagle_core::{EagleClient, MetricBuilder, Source};
use heim::units::information::byte;
use tokio::time::Duration;

pub struct Memory;

#[async_trait::async_trait]
impl Source for Memory {
    async fn produce(&mut self, client: EagleClient) {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            match heim::memory::memory().await {
                Err(e) => {
                    tracing::error!(
                        target = client.origin().instance_id(),
                        "Failed to load memory info: {}",
                        e
                    );
                }

                Ok(memory) => {
                    if let Err(e) = client
                        .send_metrics(vec![
                            MetricBuilder::gauge(
                                "host",
                                "memory_total_bytes",
                                memory.total().get::<byte>() as f64,
                            )
                            .build(),
                            MetricBuilder::gauge(
                                "host",
                                "memory_free_bytes",
                                memory.total().get::<byte>() as f64,
                            )
                            .build(),
                            MetricBuilder::gauge(
                                "host",
                                "memory_available_bytes",
                                memory.total().get::<byte>() as f64,
                            )
                            .build(),
                            MetricBuilder::gauge(
                                "host",
                                "memory_used_bytes",
                                memory.total().get::<byte>() as f64,
                            )
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
