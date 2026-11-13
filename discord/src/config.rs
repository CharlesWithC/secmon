use serde::Deserialize;
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
    pub webhook: String,

    /// Optional message content.
    #[serde(default)]
    pub message: String,
}
