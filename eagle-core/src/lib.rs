pub mod config;

use chrono::{DateTime, Utc};
use eyre::WrapErr;
use serde::Serialize;
use serde_json::Value;
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

    pub fn send_metric(&self, origin: Arc<Origin>, metric: Metric) -> eyre::Result<()> {
        self.send_metrics(origin, vec![metric])
    }

    pub fn send_metrics(&self, origin: Arc<Origin>, metrics: Vec<Metric>) -> eyre::Result<()> {
        for metric in metrics {
            self.inner
                .send(EagleEvent {
                    origin: origin.clone(),
                    event: Event::Metric(metric),
                })
                .wrap_err("Main eagle engine shut down")?;
        }

        Ok(())
    }

    pub fn send_log_with_metadata<L, M>(
        &self,
        origin: Arc<Origin>,
        log: L,
        metadata: M,
    ) -> eyre::Result<()>
    where
        L: Serialize,
        M: Serialize,
    {
        self.inner
            .send(EagleEvent {
                origin,
                event: Event::Log(Log {
                    inner: Arc::new(
                        serde_json::to_value(log).wrap_err("Error when serializing log object")?,
                    ),
                    metadata: serde_json::to_value(metadata)
                        .wrap_err("Error when serializing metadata object")?,
                }),
            })
            .wrap_err("Main eagle engine shut down")
    }

    pub fn send_log<L>(&self, origin: Arc<Origin>, log: L) -> eyre::Result<()>
    where
        L: Serialize,
    {
        self.send_log_with_metadata(origin, log, Value::Object(Default::default()))
    }
}

pub struct EagleClient {
    pub origin: Arc<Origin>,
    pub endpoint: EagleEndpoint,
}

impl EagleClient {
    pub async fn send_metric(&self, metric: Metric) -> eyre::Result<()> {
        self.endpoint.send_metric(self.origin.clone(), metric)
    }

    pub async fn send_metrics(&self, metrics: Vec<Metric>) -> eyre::Result<()> {
        self.endpoint.send_metrics(self.origin.clone(), metrics)
    }

    pub async fn send_log_with_metadata<L, M>(&self, log: L, metadata: M) -> eyre::Result<()>
    where
        L: Serialize,
        M: Serialize,
    {
        self.endpoint
            .send_log_with_metadata(self.origin.clone(), log, metadata)
    }

    pub async fn send_log<L>(&self, log: L) -> eyre::Result<()>
    where
        L: Serialize,
    {
        self.endpoint.send_log(self.origin.clone(), log)
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
    Log(Log),
    Tick,
    Shutdown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct Log {
    pub inner: Arc<Value>,
    pub metadata: Value,
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
pub struct MetricEvent {
    pub origin: Arc<Origin>,
    pub metric: Arc<Metric>,
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

    pub fn filter_by_category<F>(fun: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        Self::new(move |_, m| fun(m.category.as_str()))
    }

    pub fn category_equals(name: impl AsRef<str> + Send + Sync + 'static) -> Self {
        Self::filter_by_source_name(move |category| category == name.as_ref())
    }
}

pub struct EagleStream<A> {
    inner: mpsc::Receiver<EagleMsg<A>>,
}

impl<A> EagleStream<A> {
    pub async fn recv(&mut self) -> Recv<A> {
        if let Some(msg) = self.inner.recv().await {
            return Recv::Available(msg);
        }

        Recv::Disconnected
    }
}

pub enum EagleMsg<A> {
    Msg(A),
    Tick,
    Shutdown,
}

pub enum Recv<A> {
    Available(EagleMsg<A>),
    Disconnected,
}

#[derive(Clone)]
pub struct EagleSink<A> {
    inner: mpsc::Sender<EagleMsg<A>>,
}

impl<A> EagleSink<A> {
    pub async fn shutdown(&self) {
        let _ = self.inner.send(EagleMsg::Shutdown).await;
    }

    pub async fn send_msg(&self, msg: A) -> bool {
        self.inner.send(EagleMsg::Msg(msg)).await.is_ok()
    }

    pub async fn send_tick(&self) -> bool {
        self.inner.send(EagleMsg::Tick).await.is_ok()
    }
}

pub fn eagle_channel<A>(size: usize) -> (EagleSink<A>, EagleStream<A>) {
    let (send_inner, recv_inner) = mpsc::channel(size);

    (
        EagleSink { inner: send_inner },
        EagleStream { inner: recv_inner },
    )
}

#[async_trait::async_trait]
pub trait MetricSink {
    async fn process(
        &mut self,
        origin: Arc<Origin>,
        stream: EagleStream<MetricEvent>,
    ) -> eyre::Result<()>;
}

#[async_trait::async_trait]
pub trait Source {
    async fn produce(&mut self, client: EagleClient);
}

pub trait Transformer {
    fn transform(&mut self, origin: Arc<Origin>, metric: Metric) -> Option<Metric>;
}
