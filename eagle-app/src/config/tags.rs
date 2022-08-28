use serde::Deserialize;

#[derive(Deserialize)]
pub struct TagsConfig {
    pub disks: Vec<String>,
}
