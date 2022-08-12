use std::time::Duration;

pub struct StackDriverMetricsOptions {
    pub(crate) credentials_path: Option<String>,
    pub(crate) batch_size: usize,
    pub(crate) period: Duration,
    pub(crate) retries: usize,
}

impl Default for StackDriverMetricsOptions {
    fn default() -> Self {
        Self {
            credentials_path: None,
            batch_size: 200,
            period: Duration::from_secs(10),
            retries: 3,
        }
    }
}

impl StackDriverMetricsOptions {
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
}
