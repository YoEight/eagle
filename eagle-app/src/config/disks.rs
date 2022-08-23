use serde::Deserialize;

#[derive(Deserialize)]
pub struct DisksConfig {
    pub disks: Vec<String>,
}
