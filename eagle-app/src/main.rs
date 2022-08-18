use eagle::{
    engines::VSpec,
    sinks::Console,
    sources::{Disks, Load, Memory},
};
use eagle_core::{
    config::{Configuration, SinkConfig, SourceConfig},
    MetricFilter,
};
use eagle_google::{sinks::StackDriverMetrics, StackDriverMetricsOptions};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let gcp_options = StackDriverMetricsOptions::new("project_id");
    let conf = Configuration::default()
        .register_sink("console", SinkConfig::default(), Console)
        .register_sink(
            "gcp_metrics",
            SinkConfig::default().filter(MetricFilter::category_equals("host")),
            StackDriverMetrics::new(gcp_options)?,
        )
        .register_source(
            "disks",
            SourceConfig::default(),
            Disks::new(vec!["nvme0n1".to_string()]),
        )
        .register_source("load", SourceConfig::default(), Load)
        .register_source("memory", SourceConfig::default(), Memory);

    let process = VSpec::start(conf);

    process.wait_until_complete().await;

    Ok(())
}
