use futures::{channel::mpsc, SinkExt};
use std::sync::Arc;

#[derive(Clone)]
pub struct EagleEndpoint {
    inner: mpsc::UnboundedSender<EagleEvent>,
}

impl EagleEndpoint {
    pub fn new(inner: mpsc::UnboundedSender<EagleEvent>) -> Self {
        Self { inner }
    }

    pub async fn send_metric(&self, origin: Origin, metric: Metric) -> bool {
        self.send_metrics(origin, vec![metric]).await
    }

    pub async fn send_metrics(&self, origin: Origin, metrics: Vec<Metric>) -> bool {
        let msgs = metrics.into_iter().map(|m| {
            Ok(EagleEvent {
                origin,
                event: Event::Metric(m),
            })
        });

        let mut msgs = futures::stream::iter(msgs);

        self.inner.clone().send_all(&mut msgs).await.is_ok()
    }
}

pub struct EagleClient {
    origin: Origin,
    endpoint: EagleEndpoint,
}

impl EagleClient {
    pub async fn send_metric(&self, metric: Metric) -> bool {
        self.endpoint.send_metric(self.origin, metric).await
    }

    pub async fn send_metrics(&self, metrics: Vec<Metric>) -> bool {
        self.endpoint.send_metrics(self.origin, metrics).await
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Origin(pub uuid::Uuid);

impl Origin {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug)]
pub struct EagleEvent {
    pub origin: Origin,
    pub event: Event,
}

#[derive(Debug)]
pub enum Event {
    Metric(Metric),
    Tick,
}

/// We should have Metric and Runtime related metric info like
/// what source generated the metric.
#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub source: String, // FIXME - That field shouldn't be here.
    pub value: f64,
    // TODO - Add tags, timestamp and stuff.
}

#[derive(Clone)]
pub enum MetricEvent {
    Metric(Arc<Metric>),
    Tick,
}

pub struct MetricFilter {
    inner: Box<dyn Fn(&Metric) -> bool + Send + Sync>,
}

impl MetricFilter {
    pub fn is_handled(&self, event: &Metric) -> bool {
        (self.inner)(event)
    }

    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(&Metric) -> bool + Send + Sync + 'static,
    {
        Self {
            inner: Box::new(fun),
        }
    }

    pub fn no_filter() -> Self {
        MetricFilter::new(|_| true)
    }

    pub fn filter_by_source_name<F>(fun: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        Self::new(move |m| fun(m.source.as_str()))
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
    async fn produce(self, client: EagleClient);
}
