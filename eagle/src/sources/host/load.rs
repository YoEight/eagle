use eagle_core::{EagleClient, MetricBuilder, Source};
use eyre::WrapErr;
use heim::units::ratio::ratio;
use tokio::time::Duration;

pub struct Load;

#[async_trait::async_trait]
impl Source for Load {
    async fn produce(&mut self, client: EagleClient) -> eyre::Result<()> {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            let loadavg = heim::cpu::os::unix::loadavg()
                .await
                .wrap_err("Failed to load average info")?;

            client
                .send_metrics(vec![
                    MetricBuilder::gauge("host", "load1", loadavg.0.get::<ratio>() as f64).build(),
                    MetricBuilder::gauge("host", "load5", loadavg.1.get::<ratio>() as f64).build(),
                    MetricBuilder::gauge("host", "load15", loadavg.2.get::<ratio>() as f64).build(),
                ])
                .await?;

            clock.tick().await;
        }
    }
}
