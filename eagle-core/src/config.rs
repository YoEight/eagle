use crate::{MetricFilter, MetricSink, Origin, Source, Transformer};

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

pub struct TransformerConfig {
    pub filter: MetricFilter,
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            filter: MetricFilter::no_filter(),
        }
    }
}

impl SinkConfig {
    pub fn filter(self, filter: MetricFilter) -> Self {
        Self { filter, ..self }
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
    pub transformers: Vec<TransformerDecl>,
}

pub struct TransformerDecl {
    pub origin: Origin,
    pub config: TransformerConfig,
    pub transformer: Box<dyn Transformer + Send + 'static>,
}

impl Configuration {
    pub fn register_source<S>(&mut self, name: impl AsRef<str>, config: SourceConfig, source: S)
    where
        S: Source + Send + 'static,
    {
        let origin = Origin::new(name);

        self.sources.push(SourceDecl {
            origin,
            config,
            source: Box::new(source),
        });
    }

    pub fn register_sink<S>(&mut self, name: impl AsRef<str>, config: SinkConfig, sink: S)
    where
        S: MetricSink + Send + 'static,
    {
        self.sinks.push(SinkDecl {
            origin: Origin::new(name),
            config,
            sink: Box::new(sink),
        });
    }

    pub fn register_transformer<T>(
        &mut self,
        name: impl AsRef<str>,
        config: TransformerConfig,
        transformer: T,
    ) where
        T: Transformer + Send + 'static,
    {
        self.transformers.push(TransformerDecl {
            origin: Origin::new(name),
            config,
            transformer: Box::new(transformer),
        });
    }
}
