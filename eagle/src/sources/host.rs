use tokio::time::Duration;

use eagle_core::{EagleClient, Metric, Source};
use futures::StreamExt;
use heim::units::information::byte;

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
    async fn produce(self, client: EagleClient) {
        let mut clock = tokio::time::interval(Duration::from_secs(3));

        loop {
            match heim_disk::io_counters().await {
                Err(_) => {
                    // TODO - should decide what we should do in this case.
                }

                Ok(counters) => {
                    let mut counters = Box::pin(counters);

                    while let Some(item) = counters.next().await {
                        if let Ok(counter) = item {
                            // println!("Device: {:?}", counter.device_name());
                            if self
                                .disks
                                .iter()
                                .any(|d| d.as_str() == counter.device_name())
                            {
                                if !client
                                    .send_metrics(vec![
                                        Metric {
                                            name: "disk_read_bytes_total".to_string(),
                                            value: counter.read_bytes().get::<byte>() as f64,
                                        },
                                        Metric {
                                            name: "disk_reads_completed_total".to_string(),
                                            value: counter.read_count() as f64,
                                        },
                                        Metric {
                                            name: "disk_written_bytes_total".to_string(),
                                            value: counter.write_bytes().get::<byte>() as f64,
                                        },
                                        Metric {
                                            name: "disk_written_completed_total".to_string(),
                                            value: counter.write_count() as f64,
                                        },
                                    ])
                                    .await
                                {
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
