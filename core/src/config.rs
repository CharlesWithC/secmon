use gethostname::gethostname;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

/// Whether to print verbose log. Used by `vprintln!`.
pub static VERBOSE: OnceLock<bool> = OnceLock::new();
/// Parsed config.
pub static CONFIG: OnceLock<Config> = OnceLock::new();

/// `println!` wrapper for verbose mode.
#[macro_export]
macro_rules! vprintln {
    ($($arg:tt)*) => {
        // note: VERBOSE is not set if -v/--verbose is not passed
        if *VERBOSE.get().unwrap_or(&false) {
            println!($($arg)*);
        }
    };
}

/// Macro for getting hub/node/client config.
#[doc(hidden)]
#[macro_export]
macro_rules! __get_config {
    ( $field: ident ) => {
        &CONFIG
            .get()
            .expect("Unable to read global config variable")
            .$field
    };
}
pub use crate::__get_config as get_config;

pub const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const DEFAULT_PORT: u16 = 9992;

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hub: HubConfig,
    pub node: NodeConfig,
    pub client: ClientConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HubConfig {
    pub bind_ip: IpAddr,
    pub bind_port: u16,

    /// Time (in seconds) before a disconnected node is removed, if not replaced.
    pub disconnect_grace_period: u64,

    /// Whether hostnames can be assumed to be unique.
    ///
    /// If true, replace a disconnected node if it reconnects with the same hostname.
    ///
    /// Otherwise, treat all new connections as new nodes.
    pub assume_hostname_unique: bool,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            bind_ip: DEFAULT_IP,
            bind_port: DEFAULT_PORT,
            disconnect_grace_period: 300,
            assume_hostname_unique: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeConfig {
    pub hub_ip: IpAddr,
    pub hub_port: u16,

    /// Whether to automatically reconnect, if disconnected from hub
    pub reconnect: bool,

    /// Optional custom hostname.
    ///
    /// If empty or not set, system hostname is used.
    ///
    /// Note: hostname should not start with '-' to avoid conflicting with command line arguments.
    pub hostname: String,

    /// Resources to monitor.
    ///
    /// Accepted resources: "who", "wg", "auth".
    pub resources: Vec<String>,

    /// Time (in seconds) to wait when executing a command, before killing the child process. Use 0 to disable.
    pub command_execute_timeout: u64,

    /// Label-command dictionary of whitelisted commands.
    ///
    /// Label should not start with '-' to avoid conflicting with command line arguments.
    pub command_allowlist: BTreeMap<String, String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            hub_ip: DEFAULT_IP,
            hub_port: DEFAULT_PORT,
            reconnect: true,
            hostname: gethostname().to_string_lossy().into_owned(),
            resources: Vec::new(),
            command_execute_timeout: 0,
            command_allowlist: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClientConfig {
    /// Time (in seconds) to wait when node is executing a command, before aborting execution. Use 0 to disable.
    pub node_wait_timeout: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            node_wait_timeout: 0,
        }
    }
}
