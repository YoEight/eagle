mod disks;
mod google;

use std::collections::HashMap;

use eagle::{
    sinks::Console,
    sources::{Disks, Load, Memory},
};
use eagle_core::config::{Configuration, SinkConfig, SourceConfig};
use eagle_google::sinks::StackDriverMetrics;
use eyre::{bail, WrapErr};
use serde::Deserialize;
use toml::Value;

use crate::config::google::StackDriverMetricsConfig;

use self::disks::DisksConfig;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub sources: HashMap<String, SourceDefinition>,
    pub sinks: HashMap<String, SinkDefinition>,
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