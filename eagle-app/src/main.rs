use eagle::{
    engines::VSpec,
    sinks::Console,
    sources::{Disks, Load},
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
        .register_source("load", SourceConfig::default(), Load);

    let process = VSpec::start(conf);

    process.wait_until_complete().await;
}
