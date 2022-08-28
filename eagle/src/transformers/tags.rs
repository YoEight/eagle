use std::{collections::BTreeMap, sync::Arc};

use eagle_core::{Metric, Transformer};

pub struct Tags {
    tags: BTreeMap<String, String>,
}

impl Tags {
    pub fn new(tags: BTreeMap<String, String>) -> Self {
        Self { tags }
    }
}

impl Transformer for Tags {
    fn transform(
        &mut self,
        _origin: Arc<eagle_core::Origin>,
        mut metric: Metric,
    ) -> Option<Metric> {
        for (key, value) in self.tags.iter() {
            metric.tags.insert(key.clone(), value.clone());
        }

        Some(metric)
    }
}
