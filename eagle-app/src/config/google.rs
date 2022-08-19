use std::{collections::HashMap, time::Duration};

use eagle_google::{Resource, StackDriverMetricsOptions};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct StackDriverMetricsConfig {
    pub project_id: String,

    #[serde(default)]
    pub default_resource: ResourceConfig,

    #[serde(default)]
    pub mappings: Vec<MappingConfig>,

    pub credentials_path: Option<String>,

    #[serde(default = "default_retries")]
    pub retries: usize,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default = "default_period_in_secs")]
    pub period_in_secs: u64,
}

fn default_retries() -> usize {
    3
}

fn default_batch_size() -> usize {
    200
}

fn default_period_in_secs() -> u64 {
    10
}

impl StackDriverMetricsConfig {
    pub fn into_options(self) -> StackDriverMetricsOptions {
        let mut mappings = HashMap::new();

        for mapping in self.mappings {
            mappings.insert(mapping.metric_type, mapping.resource.into_resource());
        }

        StackDriverMetricsOptions::new(self.project_id)
            .default_resource(self.default_resource.into_resource())
            .resource_mappings(mappings)
            .credentials_options(self.credentials_path)
            .retries(self.retries)
            .batch_size(self.batch_size)
            .period(Duration::from_secs(self.period_in_secs))
    }
}

#[derive(Deserialize, Default)]
pub struct ResourceConfig {
    pub r#type: String,
    pub labels: HashMap<String, String>,
}

impl ResourceConfig {
    pub fn into_resource(self) -> Resource {
        Resource {
            r#type: self.r#type,
            labels: self.labels,
        }
    }
}

#[derive(Deserialize)]
pub struct MappingConfig {
    pub metric_type: String,
    pub resource: ResourceConfig,
}
