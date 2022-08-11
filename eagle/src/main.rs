mod engines;
mod sinks;
mod sources;

use engines::VSpec;

use eagle_core::config::{Configuration, SinkConfig, SourceConfig};
use sinks::Console;
use sources::Disks;

#[tokio::main]
async fn main() {
    let conf = Configuration::default()
        .register_sink("console", SinkConfig::default(), Console)
        .register_source(
            "host",
            SourceConfig::default(),
            Disks::new(vec!["nvme0n1".to_string()]),
        );

    let process = VSpec::start(conf);

    process.wait_until_complete().await;
}
