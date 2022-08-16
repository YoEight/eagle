use eagle_core::{MetricEvent, MetricSink, MetricType};

use crate::google::generated::{
    google_api::{
        label_descriptor::ValueType, metric_descriptor::MetricKind, Metric, MonitoredResource,
    },
    google_monitoring_v3::{typed_value::Value, Point, TimeInterval, TimeSeries, TypedValue},
};

use super::types::StackDriverMetricsOptions;
use chrono::{DateTime, Utc};
use eyre::WrapErr;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tonic::transport::{Channel, ClientTlsConfig};

struct CachedDate {
    time: DateTime<Utc>,
    clock: Instant,
}

impl CachedDate {
    fn new() -> Self {
        Self {
            time: Utc::now(),
            clock: Instant::now(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    fn reset(&mut self) {
        self.time = Utc::now();
        self.clock = Instant::now();
    }

    fn time(&self) -> DateTime<Utc> {
        self.time
    }
}

pub struct StackDriverMetrics {
    options: StackDriverMetricsOptions,
    clock: Instant,
    buffer: HashMap<String, TimeSeries>,
    started: CachedDate,
    channel: Channel,
}

impl StackDriverMetrics {
    pub fn new(options: StackDriverMetricsOptions) -> eyre::Result<Self> {
        // let uri = "https://monitoring.googleapis.com"
        //     .parse()
        //     .wrap_err("Error when parsing GCP monitoring URL")?;

        // let channel = Channel::builder(uri)
        //     .tls_config(ClientTlsConfig::new())?
        //     .connect()
        //     .await?;

        Ok(Self {
            options,
            clock: Instant::now(),
            buffer: Default::default(),
            started: CachedDate::new(),
            channel: todo!(),
        })
    }
}

/// According to GCP, a metric start time can't be more than 25 hours in the past.
const DURATION_25_HOURS: Duration = Duration::from_secs(25 * 3_600);

#[async_trait::async_trait]
impl MetricSink for StackDriverMetrics {
    async fn process(&mut self, event: MetricEvent) {
        if let Some(metric) = event.metric() {
            if self.started.elapsed() >= DURATION_25_HOURS {
                self.started.reset();
            }

            let metric_type = format!(
                "custom.googleapis.com/{}/metrics/{}",
                metric.category, metric.name
            );

            let resource =
                if let Some(res) = self.options.resource_mappings.get(metric.category.as_str()) {
                    MonitoredResource {
                        r#type: res.r#type.clone(),
                        labels: res.labels.clone(),
                    }
                } else {
                    MonitoredResource {
                        r#type: self.options.default_resource.r#type.clone(),
                        labels: self.options.default_resource.labels.clone(),
                    }
                };

            let metric_kind = match metric.r#type {
                MetricType::Gauge => MetricKind::Gauge,
                MetricType::Counter => MetricKind::Cumulative,
            };

            let start_time = crate::google::to_timestamp(self.started.time());
            let end_time = crate::google::to_timestamp(metric.timestamp);

            self.buffer.insert(
                metric_type.clone(),
                TimeSeries {
                    metric: Some(Metric {
                        r#type: metric_type,
                        labels: metric.tags.clone().into_iter().collect::<HashMap<_, _>>(),
                    }),
                    resource: Some(resource),
                    metadata: None,
                    metric_kind: metric_kind.into(),
                    value_type: ValueType::Int64.into(),
                    points: vec![Point {
                        interval: Some(TimeInterval {
                            end_time: Some(end_time),
                            start_time: Some(start_time),
                        }),
                        value: Some(TypedValue {
                            value: Some(Value::Int64Value(metric.value as i64)),
                        }),
                    }],
                    unit: "INT64".to_string(),
                },
            );
        }

        if self.buffer.len() == self.options.batch_size {
            while self.clock.elapsed() < self.options.period {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        if self.clock.elapsed() < self.options.period {
            return;
        }

        // TODO - Ready to send to GCP here.

        self.clock = Instant::now();
    }
}
