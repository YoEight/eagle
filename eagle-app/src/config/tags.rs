use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TagsConfig {
    #[serde(flatten)]
    pub tags: BTreeMap<String, String>,
}
