use serde::{Deserialize, de::Error};
use std::sync::OnceLock;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscordConfig {
    /// Required webhook URL.
    #[serde(deserialize_with = "not_empty")]
    pub webhook: String,

    /// Optional message content.
    #[serde(default)]
    pub message: String,
}

/// Helper function to deserialize a string that must not be empty.
fn not_empty<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    if s.is_empty() {
        return Err(D::Error::custom(format!("value must not be empty")));
    }
    Ok(s)
}
