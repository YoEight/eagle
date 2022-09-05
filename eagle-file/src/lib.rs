use serde::{
    de::{Error, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Deserialize, Debug)]
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

