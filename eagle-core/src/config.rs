use crate::{MetricFilter, MetricSink, Origin, Source};

pub struct SinkConfig {
    pub filter: MetricFilter,
}

impl Default for SinkConfig {
    fn default() -> Self {
        Self {
            filter: MetricFilter::no_filter(),
        }
    }
}

pub struct SinkDecl {
    pub origin: Origin,
    pub config: SinkConfig,
    pub sink: Box<dyn MetricSink + Send + 'static>,
}

#[derive(Default)]
pub struct SourceConfig {}

pub struct SourceDecl {
    pub origin: Origin,
    pub config: SourceConfig,
    pub source: Box<dyn Source + Send + 'static>,
}

#[derive(Default)]
pub struct Configuration {
    pub sources: Vec<SourceDecl>,
    pub sinks: Vec<SinkDecl>,
}

impl Configuration {
    pub fn register_source<S>(
        mut self,
        name: impl AsRef<str>,
        config: SourceConfig,
        source: S,
    ) -> Self
    where
        S: Source + Send + 'static,
    {
        let origin = Origin::new(name);

        self.sources.push(SourceDecl {
            origin,
            config,
            source: Box::new(source),
        });

        self
    }

    pub fn register_sink<S>(mut self, name: impl AsRef<str>, config: SinkConfig, sink: S) -> Self
    where
        S: MetricSink + Send + 'static,
    {
        self.sinks.push(SinkDecl {
            origin: Origin::new(name),
            config,
            sink: Box::new(sink),
        });

        self
    }
}
