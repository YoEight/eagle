pub mod config;

use std::sync::Arc;
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

/// We should have Metric and Runtime related metric info like
/// what source generated the metric.
#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    // TODO - Add tags, timestamp and stuff.
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
