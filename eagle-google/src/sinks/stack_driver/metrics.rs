use eagle_core::{EagleMsg, EagleStream, MetricEvent, MetricSink, MetricType, Origin, Recv};

use crate::generated::{
    google_api::{
        label_descriptor::ValueType, metric_descriptor::MetricKind, Metric, MonitoredResource,
    },
    google_monitoring_v3::{
        metric_service_client::MetricServiceClient, typed_value::Value, CreateTimeSeriesRequest,
        Point, TimeInterval, TimeSeries, TypedValue,
    },
};

use crate::types::StackDriverMetricsOptions;
use chrono::{DateTime, Utc};
use eyre::WrapErr;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tonic::{
    metadata::{Ascii, MetadataValue},
    service::Interceptor,
    transport::{Channel, ClientTlsConfig},
    Code, Request, Status,
};

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

struct GcpToken {
    auth_value: MetadataValue<Ascii>,
}

impl GcpToken {
    fn new(options: &StackDriverMetricsOptions) -> eyre::Result<Self> {
        let mut token = gouth::Builder::new().scopes(&[
            "https://www.googleapis.com/auth/cloud-platform",
            "https://www.googleapis.com/auth/monitoring",
            "https://www.googleapis.com/auth/monitoring.write",
        ]);

        if let Some(path) = options.credentials_path.as_ref() {
            token = token.file(path);
        }

        let token = token
            .build()
            .wrap_err("Error when calling Token::build()")?;
        let auth_value = token
            .header_value()
            .wrap_err("Error when creating GCP header value")?;
        let auth_value = MetadataValue::try_from(&*auth_value)
            .wrap_err("Error when creating CGP metadata value")?;

        Ok(Self { auth_value })
    }
}

impl Interceptor for GcpToken {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.auth_value.clone());

        Ok(request)
    }
}

pub struct StackDriverMetrics {
    options: StackDriverMetricsOptions,
}

impl StackDriverMetrics {
    pub fn new(options: StackDriverMetricsOptions) -> Self {
        Self { options }
    }
}

/// According to GCP, a metric start time can't be more than 25 hours in the past.
const DURATION_25_HOURS: Duration = Duration::from_secs(25 * 3_600);

#[async_trait::async_trait]
impl MetricSink for StackDriverMetrics {
    async fn process(
        &mut self,
        origin: Arc<Origin>,
        mut stream: EagleStream<MetricEvent>,
    ) -> eyre::Result<()> {
        let mut clock = Instant::now();
        let mut started = CachedDate::new();
        let mut buffer = HashMap::with_capacity(self.options.batch_size);
        let project_name = format!("projects/{}", self.options.project_id);

        let uri = "https://monitoring.googleapis.com"
            .parse()
            .wrap_err("Error when parsing GCP monitoring URL")?;

        let channel = Channel::builder(uri)
            .tls_config(ClientTlsConfig::new())?
            .connect()
            .await?;

        let gcp_token = GcpToken::new(&self.options)?;

        let mut client = MetricServiceClient::with_interceptor(channel, gcp_token);

        while let Recv::Available(event) = stream.recv().await {
            match event {
                EagleMsg::Tick => {
                    if clock.elapsed() < self.options.period || buffer.is_empty() {
                        continue;
                    }
                }
                EagleMsg::Msg(event) => {
                    let metric = event.metric;
                    if started.elapsed() >= DURATION_25_HOURS {
                        started.reset();
                    }

                    let metric_type = format!(
                        "custom.googleapis.com/{}/metrics/{}",
                        metric.category, metric.name
                    );

                    let resource = if let Some(res) =
                        self.options.resource_mappings.get(metric.category.as_str())
                    {
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

                    let end_time = crate::to_timestamp(metric.timestamp);
                    let start_time = if metric.r#type == MetricType::Gauge {
                        end_time.clone()
                    } else {
                        crate::to_timestamp(started.time())
                    };

                    buffer.insert(
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

                    if buffer.len() == self.options.batch_size {
                        while clock.elapsed() < self.options.period {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    } else {
                        continue;
                    }
                }
                EagleMsg::Shutdown => {
                    break;
                }
            }

            let series = buffer.drain().map(|t| t.1).collect::<Vec<_>>();
            let mut attempts = 1usize;

            while attempts < self.options.retries {
                if let Err(status) = client
                    .create_time_series(Request::new(CreateTimeSeriesRequest {
                        name: project_name.clone(),
                        time_series: series.clone(),
                    }))
                    .await
                {
                    if status.code() == Code::Internal || status.code() == Code::Unknown {
                        attempts += 1;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    tracing::error!(
                        target = origin.instance_id(),
                        "Error when sending time_series: {}",
                        status
                    );

                    counter!("stackdriver.metrics.failures", 1);
                    break;
                }

                counter!("stackdriver.metrics.successes", 1);
                break;
            }

            clock = Instant::now();
        }

        Ok(())
    }
}
