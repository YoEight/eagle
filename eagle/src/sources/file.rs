use eagle_core::{EagleClient, Source};
use eyre::{bail, WrapErr};
use serde::{Deserialize, Serialize};
use std::{io::SeekFrom, path::PathBuf, time::Duration};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
};

#[derive(Eq, PartialEq, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    Json,
    Text,
}

pub struct File {
    filepath: PathBuf,
    codec: Codec,
}

impl File {
    pub fn new(filepath: PathBuf, codec: Codec) -> Self {
        Self { filepath, codec }
    }
}

#[async_trait::async_trait]
impl Source for File {
    async fn produce(&mut self, client: EagleClient) -> eyre::Result<()> {
        let mut file = fs::File::open(self.filepath.as_path())
            .await
            .wrap_err_with(|| format!("Error when opening file {:?}", self.filepath.as_path()))?;

        file.seek(SeekFrom::End(0)).await.wrap_err_with(|| {
            format!(
                "Error when reaching to the end of the file '{:?}'",
                self.filepath.as_path()
            )
        })?;

        let mut lines = BufReader::with_capacity(8_192, file).lines();

        loop {
            match lines.next_line().await {
                Err(e) => bail!(
                    "Error when pulling lines out of {:?}: {}",
                    self.filepath.as_path(),
                    e
                ),

                Ok(line) => {
                    if let Some(line) = line {
                        let log = match self.codec {
                            Codec::Json => {
                                serde_json::from_str(line.as_str()).wrap_err("Invalid JSON")?
                            }
                            Codec::Text => serde_json::Value::String(line),
                        };

                        client.send_log(log).await?;

                        continue;
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}
