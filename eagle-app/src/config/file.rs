use std::path::PathBuf;

use eagle::sources::Codec;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FileConfig {
    pub filepath: PathBuf,
    pub codec: Codec,
}
