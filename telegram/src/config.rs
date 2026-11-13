use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, de::Error};
use std::sync::OnceLock;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub client: secmon::config::ClientConfig,

    pub telegram: TelegramConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramConfig {
    /// Required bot token.
    #[serde(deserialize_with = "not_empty")]
    pub token: String,

    /// ID of telegram user to accept commands from and send updates to.
    pub user_id: u64,

    /// List of command labels that may be executed through the bot.
    ///
    /// The labels should match the ones in node command allowlist.
    #[serde(default)]
    pub command_allowlist: Vec<String>,

    /// Timezone for date time in messages.
    ///
    /// If not set, use system timezone, or fall back to UTC.
    #[serde(default = "default_timezone", deserialize_with = "timezone")]
    pub timezone: Tz,

    /// Whether to disable ipv6 when communicating with telegram api.
    ///
    /// Note: ipv6 connection may timeout; enable this if that happens
    #[serde(default)]
    pub ipv4_only: bool,
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

/// Helper function to get default timezone (i.e. system timezone, or fallback to UTC).
///
/// Default timezone is used when timezone is not set or empty.
fn default_timezone() -> Tz {
    iana_time_zone::get_timezone()
        .unwrap_or_else(|_| "UTC".to_string())
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC)
}

/// Helper function to deseralize timezone.
///
/// If timezone is empty, this overwrites it with default timezone.
///
/// Note: This does not handle when timezone is not set, which is handled by `default` instead.
fn timezone<'de, D>(deserializer: D) -> Result<Tz, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(if s.is_empty() {
        default_timezone()
    } else {
        s.parse::<Tz>().map_err(D::Error::custom)?
    })
}
