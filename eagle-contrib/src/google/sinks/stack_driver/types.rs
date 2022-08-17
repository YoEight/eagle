use std::{collections::HashMap, time::Duration};

pub struct Resource {
    pub r#type: String,
    pub labels: HashMap<String, String>,
}

impl Resource {
    pub fn new(r#type: impl AsRef<str>) -> Self {
        Self {
            r#type: r#type.as_ref().to_string(),
            labels: Default::default(),
        }
    }

    pub fn add_label(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.labels
            .insert(name.as_ref().to_string(), value.as_ref().to_string());

        self
    }
}

pub struct StackDriverMetricsOptions {
    pub(crate) project_id: String,
    pub(crate) credentials_path: Option<String>,
    pub(crate) batch_size: usize,
    pub(crate) period: Duration,
    pub(crate) retries: usize,
    pub(crate) default_resource: Resource,
    pub(crate) resource_mappings: HashMap<String, Resource>,
}

impl StackDriverMetricsOptions {
    pub fn new(project_id: impl AsRef<str>) -> Self {
        Self {
            project_id: project_id.as_ref().to_string(),
            credentials_path: None,
            batch_size: 200,
            period: Duration::from_secs(10),
            retries: 3,
            default_resource: Resource {
                r#type: Default::default(),
                labels: Default::default(),
            },
            resource_mappings: Default::default(),
        }
    }

    pub fn credentials(self, path: impl AsRef<str>) -> Self {
        Self {
            credentials_path: Some(path.as_ref().to_string()),
            ..self
        }
    }

    pub fn credentials_options(self, credentials_path: Option<String>) -> Self {
        Self {
            credentials_path,
            ..self
        }
    }

    pub fn batch_size(self, batch_size: usize) -> Self {
        Self { batch_size, ..self }
    }

    pub fn period(self, period: Duration) -> Self {
        Self { period, ..self }
    }

    pub fn retries(self, retries: usize) -> Self {
        Self { retries, ..self }
    }

    pub fn map_resource_to(mut self, r#type: impl AsRef<str>, resource: Resource) -> Self {
        self.resource_mappings
            .insert(r#type.as_ref().to_string(), resource);

        self
    }
}
