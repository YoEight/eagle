use eagle::{
    engines::VSpec,
    sinks::Console,
    sources::{Disks, Load, Memory},
};
use eagle_core::config::{Configuration, SinkConfig, SourceConfig};

#[tokio::main]
async fn main() {
    let conf = Configuration::default()
        .register_sink("console", SinkConfig::default(), Console)
        .register_source(
            "disks",
            SourceConfig::default(),
            Disks::new(vec!["nvme0n1".to_string()]),
        )
        .register_source("load", SourceConfig::default(), Load)
        .register_source("memory", SourceConfig::default(), Memory);

    let process = VSpec::start(conf);

    process.wait_until_complete().await;
}
