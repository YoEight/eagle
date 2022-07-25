use eagle_core::{Metric, Source};
use futures::StreamExt;
use heim::units::information::byte;

struct Disks {
    disks: Vec<String>,
}

impl Disks {
    pub fn new(disks: Vec<String>) -> Self {
        Self { disks }
    }
}

#[async_trait::async_trait]
impl Source for Disks {
    async fn produce(self, endpoint: eagle_core::EagleEndpoint) {
        match heim_disk::io_counters().await {
            Err(_) => {
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
                            if !endpoint
                                .send_metrics(vec![
                                    Metric {
                                        name: "disk_read_bytes_total".to_string(),
                                        source: "hosts".to_string(), // FIXME - This value should be dynamically allocated.
                                        value: counter.read_bytes().get::<byte>() as f64,
                                    },
                                    Metric {
                                        name: "disk_reads_completed_total".to_string(),
                                        source: "hosts".to_string(), // FIXME - This value should be dynamically allocated.
                                        value: counter.read_count() as f64,
                                    },
                                    Metric {
                                        name: "disk_written_bytes_total".to_string(),
                                        source: "hosts".to_string(), // FIXME - This value should be dynamically allocated.
                                        value: counter.write_bytes().get::<byte>() as f64,
                                    },
                                    Metric {
                                        name: "disk_written_completed_total".to_string(),
                                        source: "hosts".to_string(), // FIXME - This value should be dynamically allocated.
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
    }
}
