use eagle_core::{EagleClient, MetricBuilder, Source};
use eyre::WrapErr;
use heim::units::information::byte;
use tokio::time::Duration;

pub struct Memory;

#[async_trait::async_trait]
impl Source for Memory {
    async fn produce(&mut self, client: EagleClient) -> eyre::Result<()> {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            let memory = heim::memory::memory()
                .await
                .wrap_err("Failed to load memory info")?;

            client
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
                .await?;

            clock.tick().await;
        }
    }
}
