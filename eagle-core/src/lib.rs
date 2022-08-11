pub mod config;

use chrono::{DateTime, Utc};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct EagleEndpoint {
    inner: mpsc::UnboundedSender<EagleEvent>,
}

impl EagleEndpoint {
    pub fn new(inner: mpsc::UnboundedSender<EagleEvent>) -> Self {
        Self { inner }
    }

    pub fn send_metric(&self, origin: Arc<Origin>, metric: Metric) -> bool {
        self.send_metrics(origin, vec![metric])
    }

    pub fn send_metrics(&self, origin: Arc<Origin>, metrics: Vec<Metric>) -> bool {
        for metric in metrics {
            if self
                .inner
                .send(EagleEvent {
                    origin: origin.clone(),
                    event: Event::Metric(metric),
                })
                .is_err()
            {
                return false;
            }
        }

        true
    }
}

pub struct EagleClient {
    pub origin: Arc<Origin>,
    pub endpoint: EagleEndpoint,
}

impl EagleClient {
    pub async fn send_metric(&self, metric: Metric) -> bool {
        self.endpoint.send_metric(self.origin.clone(), metric)
    }

    pub async fn send_metrics(&self, metrics: Vec<Metric>) -> bool {
        self.endpoint.send_metrics(self.origin.clone(), metrics)
    }

    pub fn origin(&self) -> &Origin {
        self.origin.as_ref()
    }
}

#[derive(Debug)]
pub struct Origin {
    pub id: Uuid,
    pub name: String,
    pub instance_id: String,
}

impl Origin {
    pub fn new(name: impl AsRef<str>) -> Self {
        let id = Uuid::new_v4();
        let instance_id = format!("source-{}:{}", name.as_ref(), id);

        Self {
            id,
            name: name.as_ref().to_string(),
            instance_id,
        }
    }

    pub fn instance_id(&self) -> &str {
        self.instance_id.as_str()
    }
}

#[derive(Debug)]
pub struct EagleEvent {
    pub origin: Arc<Origin>,
    pub event: Event,
}

#[derive(Debug)]
pub enum Event {
    Metric(Metric),
    Tick,
    Shutdown,
}

#[derive(Debug, Copy, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
}

/// We should have Metric and Runtime related metric info like
/// what source generated the metric.
#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub r#type: MetricType,
    pub category: String,
    pub tags: BTreeMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

pub struct MetricBuilder {
    name: String,
    value: f64,
    r#type: MetricType,
    category: String,
    tags: BTreeMap<String, String>,
    timestamp: DateTime<Utc>,
}

impl MetricBuilder {
    pub fn counter(category: impl AsRef<str>, name: impl AsRef<str>, value: f64) -> Self {
        Self {
            name: name.as_ref().to_string(),
            value,
            r#type: MetricType::Counter,
            category: category.as_ref().to_string(),
            tags: Default::default(),
            timestamp: Utc::now(),
        }
    }

    pub fn gauge(category: impl AsRef<str>, name: impl AsRef<str>, value: f64) -> Self {
        Self {
            name: name.as_ref().to_string(),
            value,
            r#type: MetricType::Gauge,
            category: category.as_ref().to_string(),
            tags: Default::default(),
            timestamp: Utc::now(),
        }
    }

    pub fn add_tag(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.tags
            .insert(name.as_ref().to_string(), value.as_ref().to_string());

        self
    }

    pub fn tags(self, tags: BTreeMap<String, String>) -> Self {
        Self { tags, ..self }
    }

    pub fn build(self) -> Metric {
        Metric {
            name: self.name,
            value: self.value,
            r#type: self.r#type,
            category: self.category,
            tags: self.tags,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Clone)]
pub enum MetricEvent {
    Metric {
        origin: Arc<Origin>,
        metric: Arc<Metric>,
    },
    Tick,
}

pub struct MetricFilter {
    inner: Box<dyn Fn(&Origin, &Metric) -> bool + Send + Sync>,
}

impl MetricFilter {
    pub fn is_handled(&self, origin: &Origin, event: &Metric) -> bool {
        (self.inner)(origin, event)
    }

    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(&Origin, &Metric) -> bool + Send + Sync + 'static,
    {
        Self {
            inner: Box::new(fun),
        }
    }

    pub fn no_filter() -> Self {
        MetricFilter::new(|_, _| true)
    }

    pub fn filter_by_source_name<F>(fun: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        Self::new(move |o, _| fun(o.name.as_str()))
    }

    pub fn source_name_equals(name: impl AsRef<str> + Send + Sync + 'static) -> Self {
        Self::filter_by_source_name(move |source_name| source_name == name.as_ref())
    }

    pub fn source_name_starts_with(name: impl AsRef<str> + Send + Sync + 'static) -> Self {
        Self::filter_by_source_name(move |source_name| source_name.starts_with(name.as_ref()))
    }

    pub fn source_name_ends_with(name: impl AsRef<str> + Send + Sync + 'static) -> Self {
        Self::filter_by_source_name(move |source_name| source_name.ends_with(name.as_ref()))
    }
}

#[async_trait::async_trait]
pub trait MetricSink {
    async fn process(&mut self, event: MetricEvent);
}

#[async_trait::async_trait]
pub trait Source {
    async fn produce(&mut self, client: EagleClient);
}
