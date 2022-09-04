use eagle_core::{EagleClient, Source};
use eyre::WrapErr;
use std::{io::SeekFrom, path::PathBuf, time::Duration};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
};

#[derive(Eq, PartialEq)]
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
            if let Some(line) = lines.next_line().await.wrap_err_with(|| {
                format!(
                    "Error when pulling lines out of {:?}",
                    self.filepath.as_path()
                )
            })? {
                // TODO - Procede on whether raw text or pass json.
                continue;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
