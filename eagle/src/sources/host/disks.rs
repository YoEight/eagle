use std::collections::BTreeMap;

use eagle_core::{EagleClient, MetricBuilder, Source};
use futures::StreamExt;
use heim::units::information::byte;
use tokio::time::Duration;

pub struct Disks {
    disks: Vec<String>,
}

impl Disks {
    pub fn new(disks: Vec<String>) -> Self {
        Self { disks }
    }
}

#[async_trait::async_trait]
impl Source for Disks {
    async fn produce(&mut self, client: EagleClient) {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            match heim_disk::io_counters().await {
                Err(e) => {
                    tracing::error!(
                        target = client.origin().instance_id(),
                        "Unexpected error when loading disk info: {}",
                        e
                    );
                    // TODO - should decide what we should do in this case.
                }

                Ok(counters) => {
                    let mut counters = Box::pin(counters);

                    while let Some(item) = counters.next().await {
                        if let Ok(counter) = item {
                            if self
                                .disks
                                .iter()
                                .any(|d| d.as_str() == counter.device_name())
                            {
                                let mut tags = BTreeMap::new();

                                tags.insert(
                                    "device_name".to_string(),
                                    counter.device_name().to_str().unwrap().to_string(),
                                );

                                if let Err(e) = client
                                    .send_metrics(vec![
                                        MetricBuilder::gauge(
                                            "host",
                                            "disk_read_bytes_total",
                                            counter.read_bytes().get::<byte>() as f64,
                                        )
                                        .tags(tags.clone())
                                        .build(),
                                        MetricBuilder::gauge(
                                            "host",
                                            "disk_reads_completed_total",
                                            counter.read_count() as f64,
                                        )
                                        .tags(tags.clone())
                                        .build(),
                                        MetricBuilder::gauge(
                                            "host",
                                            "disk_written_bytes_total",
                                            counter.write_bytes().get::<byte>() as f64,
                                        )
                                        .tags(tags)
                                        .build(),
                                        MetricBuilder::gauge(
                                            "host",
                                            "disk_written_completed_total",
                                            counter.write_count() as f64,
                                        )
                                        .build(),
                                    ])
                                    .await
                                {
                                    tracing::error!(
                                        target = client.origin().instance_id(),
                                        "Unexpected error when sending metrics: {}",
                                        e
                                    );

                                    break;
                                }
                            }
                        }
                    }
                }
            }

            clock.tick().await;
        }
    }
}
