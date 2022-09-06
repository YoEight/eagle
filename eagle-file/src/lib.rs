use eagle_core::{EagleClient, Source};
use eyre::{bail, WrapErr};
use glob::glob;
use serde::{
    de::{Error, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, path::PathBuf, time::Duration};
use tokio::{select, sync::mpsc};

#[derive(Deserialize, Debug, Clone)]
pub struct FileConfig {
    #[serde(deserialize_with = "deserialize_patterns")]
    pub includes: Vec<glob::Pattern>,
    #[serde(deserialize_with = "deserialize_patterns")]
    pub excludes: Vec<glob::Pattern>,
}

struct PatternVisitor;

impl<'de> Visitor<'de> for PatternVisitor {
    type Value = Vec<glob::Pattern>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expecting unix filepath glob pattern")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut patterns = Vec::new();
        while let Some(path) = seq.next_element::<&str>()? {
            match glob::Pattern::new(path) {
                Ok(pat) => {
                    patterns.push(pat);
                }

                Err(e) => return Err(A::Error::custom(e)),
            }
        }

        Ok(patterns)
    }
}

fn deserialize_patterns<'de, D>(deserializer: D) -> Result<Vec<glob::Pattern>, D::Error>
where
    D: Deserializer<'de>,
{
    todo!()
}

pub struct File {
    config: FileConfig,
}

impl File {
    pub fn new(config: FileConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Source for File {
    async fn produce(&mut self, client: EagleClient) -> eyre::Result<()> {
        let (sender, mut mail) = mpsc::unbounded_channel::<Msg>();
        let config = self.config.clone();
        let client = FileClient {
            inner: sender.clone(),
        };

        let mut clock = tokio::time::interval(Duration::from_millis(500));
        let handle = std::thread::spawn(move || discovery(client, config));

        loop {
            select! {
                msg = mail.recv() => {
                    if let Some(msg) = msg {
                        match msg {
                            Msg::Files(_files) => {

                            }
                        }
                        continue;
                    }

                    bail!("File discovery thread existed");
                }
                _ = clock.tick() => {
                    if handle.is_finished() {
                        match handle.join() {
                            Err(e) => {
                                bail!("File discovery thread exited with a platform specific error: {:?}", e.type_id());
                            }

                            Ok(outcome) => {
                                match outcome {
                                    Err(e) => {
                                        bail!("File discovery thread exited with an error: {}", e);
                                    }

                                    Ok(_) => {
                                        bail!("File descovery thread exited");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

enum Msg {
    Files(Vec<PathBuf>),
}

struct FileClient {
    inner: mpsc::UnboundedSender<Msg>,
}

impl FileClient {
    fn new(inner: mpsc::UnboundedSender<Msg>) -> Self {
        Self { inner }
    }

    fn send_files(&self, files: Vec<PathBuf>) -> eyre::Result<()> {
        if self.inner.send(Msg::Files(files)).is_err() {
            bail!("Nothing is listening to file endpoint anymore");
        }

        Ok(())
    }
}

fn list_files(config: &FileConfig) -> Vec<PathBuf> {
    config
        .includes
        .iter()
        .flat_map(|include_pattern| {
            glob(include_pattern.as_str())
                .expect("Impossible as we pre-checked the validity of the pattern")
                .filter_map(|value| value.ok())
        })
        .filter(|filepath| {
            !config
                .excludes
                .iter()
                .any(|exclude_pattern| exclude_pattern.matches_path(filepath.as_path()))
        })
        .collect()
}

fn discovery(client: FileClient, config: FileConfig) -> eyre::Result<()> {
    loop {
        let files = list_files(&config);

        client.send_files(files)?;

        std::thread::sleep(Duration::from_secs(60));
    }
}
