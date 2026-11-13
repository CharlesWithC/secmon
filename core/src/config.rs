use gethostname::gethostname;
use serde::{Deserialize, Deserializer, de::Error};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

use crate::crypto::Key;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 9992;

/// Macro for getting hub/node/client config.
///
/// Note: `CONFIG` must be imported locally.
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
#[serde(default)]
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
    ///
    /// Note: We store {key: hostname} in memory to facilitate efficient lookup on new connections.
    #[serde(deserialize_with = "inverted_btree")]
    pub node_whitelist: BTreeMap<Key, String>,
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
    #[serde(default = "default_hostname", deserialize_with = "hostname")]
    pub hostname: String,

    /// Optional group name.
    ///
    /// If empty or not set, no group is set.
    ///
    /// Note: group name should not start with '-' to avoid conflicting with command line arguments.
    #[serde(deserialize_with = "empty_as_none")]
    pub group: Option<String>,

    /// Resources to monitor.
    ///
    /// Accepted resources: "who", "wg", "auth".
    pub resources: Vec<String>,

    /// Time (in seconds) to wait when executing a command, before killing the child process. Use 0 to disable.
    pub command_execute_timeout: u64,

    /// Label-command dictionary of whitelisted commands.
    ///
    /// Label should not start with '-' to avoid conflicting with command line arguments.
    ///
    /// Command output should be smaller than 63.98MB, otherwise transmitting the result might cause a panic.
    /// This is only applicable to non-streamed output, and the size is set by `NOISE_MAX_CHUNK` in `crypto.rs`.
    #[serde(deserialize_with = "btree_key_no_leading_dash")]
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
            group: None,
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

    /// Key to order nodes, choose from: serial, address, hostname.
    ///
    /// Note: group takes precedence in ordering, i.e. nodes are sorted by group first, then by the key below;
    /// string order is used for address and hostname.
    pub node_order_key: NodeOrderKey,

    /// List of node hostnames to override node order.
    ///
    /// Deserialized as a dictionary of `{hostname: index}` for efficient sorting.
    ///
    /// Nodes in list will have fixed order, and nodes not in list will be placed after nodes in list,
    /// sorted by 'node_order_key'.
    #[serde(deserialize_with = "list_to_btree")]
    pub node_order_override: BTreeMap<String, usize>,

    /// List of groups to override group order.
    ///
    /// Deserialized as a dictionary of `{group: index}` for efficient sorting.
    ///
    /// Groups in list will have fixed order, and groups not in list will be placed after groups in list,
    /// sorted by string order.
    #[serde(deserialize_with = "list_to_btree")]
    pub group_order_override: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeOrderKey {
    Serial,
    Address,
    Hostname,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            node_wait_timeout: 0,
            node_order_key: NodeOrderKey::Hostname,
            node_order_override: BTreeMap::new(),
            group_order_override: BTreeMap::new(),
        }
    }
}

/// Helper function to deserialize empty string as `None`.
fn empty_as_none<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(d)?;
    Ok(s.filter(|s| !s.is_empty()))
}

/// Helper function to deserialize `{k: v}` btree as `{v: k}`.
fn inverted_btree<'de, D, K, V>(d: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de> + Ord,
{
    Ok(BTreeMap::<V, K>::deserialize(d)?
        .into_iter()
        .map(|(v, k)| (k, v))
        .collect())
}

/// Helper function to deserailize `[v]` list as `{v: idx}` btree.
///
/// Note: Only first entry is taken for duplicate entries.
fn list_to_btree<'de, D, V>(deserializer: D) -> Result<BTreeMap<V, usize>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de> + Ord,
{
    let l = Vec::<V>::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for (idx, v) in l.into_iter().enumerate() {
        map.entry(v).or_insert(idx);
    }
    Ok(map)
}

/// Helper function to deserialize string that should not have a leading dash.
///
/// A deserialization error is raised if the value leads with "-".
fn no_leading_dash<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.starts_with('-') {
        return Err(D::Error::custom(format!(
            "value must not start with '-': {s:?}"
        )));
    }
    Ok(s)
}

/// Helper function to deserialize `{k: v}` btree where `k` should not have a leading dash.
fn btree_key_no_leading_dash<'de, D, V>(deserializer: D) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de> + Ord,
{
    let map = BTreeMap::<String, V>::deserialize(deserializer)?;
    for (k, _) in &map {
        if k.starts_with('-') {
            return Err(D::Error::custom(format!(
                "value must not start with '-': {k:?}"
            )));
        }
    }
    Ok(map)
}

/// Helper function to get default hostname.
///
/// Default hostname is used when hostname is not set or empty.
fn default_hostname() -> String {
    gethostname().to_string_lossy().into_owned()
}

/// Helper function to deseralize hostname.
///
/// If hostname is empty, this overwrites it with default hostname.
///
/// Note: This does not handle when hostname is not set, which is handled by `default` instead.
fn hostname<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = no_leading_dash(deserializer)?;
    Ok(if s.is_empty() { default_hostname() } else { s })
}
