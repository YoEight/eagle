use std::sync::Arc;

pub enum EagleEvent {
    Metric(Metric),
}

pub struct Metric {
    pub name: String,
    pub source: String,
}

#[derive(Clone)]
pub enum MetricEvent {
    Metric(Arc<Metric>),
    None,
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

    fn filter(&self) -> MetricFilter;
}
