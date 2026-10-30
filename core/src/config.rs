use gethostname::gethostname;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

use crate::crypto::Key;

/// Whether to print verbose log. Used by `vprintln!`.
pub static VERBOSE: OnceLock<bool> = OnceLock::new();
/// Parsed config.
pub static CONFIG: OnceLock<Config> = OnceLock::new();

const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 9992;

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

    /// Noise protocol configuration.
    pub noise: HubNoise,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            bind_ip: DEFAULT_IP,
            bind_port: DEFAULT_PORT,
            disconnect_grace_period: 300,
            assume_hostname_unique: true,
            noise: HubNoise::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HubNoise {
    /// Whether to enable noise protocol.
    ///
    /// If false, then data is transmitted in plaintext.
    ///
    /// Note: keys in configuration are still parsed even if `enable=false`.
    /// Use empty strings or omit attributes for keys to avoid parsing errors.
    pub enable: bool,

    /// Required private key for hub.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub private_key: Key,

    /// Optional preshared key. This may be generated with `secmon genkey psk`.
    ///
    /// If specified, then all nodes must use the same preshared key, and
    /// authenticated encryption is enabled from the first message.
    ///
    /// This allows blocking external nodes even without `use_whitelist`.
    pub preshared_key: Key,

    /// Whether to use a whitelist of nodes.
    ///
    /// If true, then `node_whitelist` must be set.
    ///
    /// If false, then any node with preshared key (if applicable) may connect to the hub.
    pub use_whitelist: bool,

    /// Hostname-publickey dictionary of whitelisted nodes.
    pub node_whitelist: BTreeMap<String, Key>,
}

impl Default for HubNoise {
    fn default() -> Self {
        Self {
            enable: false,
            private_key: Key::default(),
            preshared_key: Key::default(),
            use_whitelist: false,
            node_whitelist: BTreeMap::new(),
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

    /// Noise protocol configuration.
    pub noise: NodeNoise,
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
            noise: NodeNoise::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeNoise {
    /// Whether to enable noise protocol.
    ///
    /// If false, then data is transmitted in plaintext.
    ///
    /// Note: keys in configuration are still parsed even if `enable=false`.
    /// Use empty strings or omit attributes for keys to avoid parsing errors.
    pub enable: bool,

    /// Required private key for node.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub private_key: Key,
    /// Required public key from hub.
    pub hub_public_key: Key,

    /// Optional preshared key.
    ///
    /// The key must be the same as the one specified in hub config.
    ///
    /// If specified, then authenticated encryption is enabled from the first message.
    ///
    /// This allows blocking unwanted nodes even without `use_whitelist`.
    pub preshared_key: Key,
}

impl Default for NodeNoise {
    fn default() -> Self {
        Self {
            enable: false,
            private_key: Key::default(),
            hub_public_key: Key::default(),
            preshared_key: Key::default(),
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
