mod disks;
mod file;
mod google;
mod tags;

use std::collections::HashMap;

use eagle::{
    sinks::Console,
    sources::{Disks, File, Load, Memory},
    transformers::tags::Tags,
};
use eagle_core::config::{Configuration, SinkConfig, SourceConfig, TransformerConfig};
use eagle_google::sinks::StackDriverMetrics;
use eyre::{bail, WrapErr};
use serde::Deserialize;
use toml::Value;

use crate::config::google::StackDriverMetricsConfig;

use self::{disks::DisksConfig, file::FileConfig, tags::TagsConfig};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub sources: HashMap<String, SourceDefinition>,
    pub sinks: HashMap<String, SinkDefinition>,
    #[serde(default)]
    pub transformers: HashMap<String, TransformerDefinition>,
}

impl Config {
    pub fn build(self) -> eyre::Result<Configuration> {
        let mut config = Configuration::default();

        for (name, definition) in self.sources {
            match name.as_str() {
                "disks" => {
                    configure_disk_source(&mut config, definition)?;
                }

                "memory" => {
                    configure_memory_source(&mut config, definition);
                }

                "load" => {
                    configure_load_source(&mut config, definition);
                }

                "file" => {
                    configure_file_source(&mut config, definition)?;
                }

                unknown => bail!("Unknown source '{}'", unknown),
            }
        }

        for (name, definition) in self.sinks {
            match name.as_str() {
                "console" => {
                    configure_console_sink(&mut config, definition);
                }

                "stackdriver_metrics" => {
                    configure_stackdriver_metrics_sink(&mut config, definition)?;
                }

                unknown => bail!("Unknown source '{}'", unknown),
            }
        }

        for (name, definition) in self.transformers {
            match name.as_str() {
                "tags" => {
                    configure_tags_transformer(&mut config, definition)?;
                }
                unknown => bail!("Unknown transformer '{}'", unknown),
            }
        }

        Ok(config)
    }
}

fn configure_disk_source(
    config: &mut Configuration,
    definition: SourceDefinition,
) -> eyre::Result<()> {
    let name = definition.name.clone();
    let options = definition.parse_params::<DisksConfig>()?;

    config.register_source(name, SourceConfig::default(), Disks::new(options.disks));

    Ok(())
}

fn configure_memory_source(config: &mut Configuration, definition: SourceDefinition) {
    config.register_source(definition.name.as_str(), SourceConfig::default(), Memory);
}

fn configure_load_source(config: &mut Configuration, definition: SourceDefinition) {
    config.register_source(definition.name.as_str(), SourceConfig::default(), Load);
}

fn configure_file_source(
    config: &mut Configuration,
    definition: SourceDefinition,
) -> eyre::Result<()> {
    let name = definition.name.clone();
    let options = definition.parse_params::<FileConfig>()?;

    config.register_source(
        name,
        SourceConfig::default(),
        File::new(options.filepath, options.codec),
    );

    Ok(())
}

fn configure_console_sink(config: &mut Configuration, definition: SinkDefinition) {
    config.register_sink(definition.name.as_str(), SinkConfig::default(), Console);
}

fn configure_stackdriver_metrics_sink(
    config: &mut Configuration,
    definition: SinkDefinition,
) -> eyre::Result<()> {
    let name = definition.name.clone();
    let params = definition.parse_params::<StackDriverMetricsConfig>()?;

    config.register_sink(
        name,
        SinkConfig::default(),
        StackDriverMetrics::new(params.into_options()),
    );

    Ok(())
}

fn configure_tags_transformer(
    config: &mut Configuration,
    definition: TransformerDefinition,
) -> eyre::Result<()> {
    let name = definition.name.clone();
    let params = definition.parse_params::<TagsConfig>()?;

    config.register_transformer(name, TransformerConfig::default(), Tags::new(params.tags));

    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct SourceDefinition {
    pub name: String,
    #[serde(flatten)]
    pub params: Value,
}

impl SourceDefinition {
    pub fn parse_params<'de, P>(self) -> eyre::Result<P>
    where
        P: Deserialize<'de>,
    {
        self.params.try_into().wrap_err("Error when parsing params")
    }
}

#[derive(Deserialize, Debug)]
pub struct SinkDefinition {
    pub name: String,
    #[serde(flatten)]
    pub params: Value,
}

impl SinkDefinition {
    pub fn parse_params<'de, P>(self) -> eyre::Result<P>
    where
        P: Deserialize<'de>,
    {
        self.params.try_into().wrap_err("Error when parsing params")
    }
}

#[derive(Deserialize, Debug)]
pub struct TransformerDefinition {
    pub name: String,

    #[serde(flatten)]
    pub params: Value,
}

impl TransformerDefinition {
    pub fn parse_params<'de, P>(self) -> eyre::Result<P>
    where
        P: Deserialize<'de>,
    {
        self.params.try_into().wrap_err("Error when parsing params")
    }
}
