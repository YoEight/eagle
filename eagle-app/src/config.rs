use std::collections::HashMap;

use eagle::{
    sinks::Console,
    sources::{Disks, Load, Memory},
};
use eagle_core::config::{Configuration, SinkConfig, SourceConfig};
use eyre::{bail, eyre};
use serde::Deserialize;
use toml::value::{Table, Value};

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
    let disks = if let Some(disks) = definition.params.get("disks") {
        if let Some(disks) = disks.as_array().and_then(parse_disks_param) {
            disks
        } else {
            bail!("'disks' parameter is not a list of string")
        }
    } else {
        Vec::<String>::new()
    };

    config.register_source(
        definition.name.as_str(),
        SourceConfig::default(),
        Disks::new(disks),
    );

    Ok(())
}

fn configure_memory_source(config: &mut Configuration, definition: SourceDefinition) {
    config.register_source(definition.name.as_str(), SourceConfig::default(), Memory);
}

fn configure_load_source(config: &mut Configuration, definition: SourceDefinition) {
    config.register_source(definition.name.as_str(), SourceConfig::default(), Load);
}

fn parse_disks_param(disks: &Vec<Value>) -> Option<Vec<String>> {
    let mut fin_disks = Vec::with_capacity(disks.len());

    for value in disks {
        fin_disks.push(value.as_str()?.to_string());
    }

    Some(fin_disks)
}

fn configure_console_sink(config: &mut Configuration, definition: SinkDefinition) {
    config.register_sink(definition.name.as_str(), SinkConfig::default(), Console);
}

#[derive(Deserialize, Debug)]
pub struct SourceDefinition {
    pub name: String,
    #[serde(flatten)]
    pub params: Table,
}

#[derive(Deserialize, Debug)]
pub struct SinkDefinition {
    pub name: String,
    #[serde(flatten)]
    pub params: Table,
}
