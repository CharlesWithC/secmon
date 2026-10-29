use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use gethostname::gethostname;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

/// Whether to print verbose log. Used by `vprintln!`.
pub static VERBOSE: OnceLock<bool> = OnceLock::new();
/// Parsed config.
pub static CONFIG: OnceLock<Config> = OnceLock::new();

/// Noise protocol. XKpsk0 requires 1.5 RTT for handshake.
///
/// XK = Hub static key known; Node sends encrypted static key after DH for hub to verify.
///
/// psk0 = Preshared key is used from first message. Doable because single psk is used.
///
/// XK is chosen over KK so hub does not have to enumerate all keys to verify node's key.
pub const NOISE_PATTERN: &str = "Noise_XKpsk0_25519_ChaChaPoly_BLAKE2s";
/// Length of keys used by noise protocol.
pub const KEYLENGTH: usize = 32;

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

    /// Required private key.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub private_key: Key,
    /// Required public key.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub public_key: Key,

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
            public_key: Key::default(),
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

    /// Required private key.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub private_key: Key,
    /// Required public key.
    ///
    /// A keypair may be generated with `secmon genkey pair`.
    pub public_key: Key,

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
            public_key: Key::default(),
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

/// Key with base64 encoding and decoding.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Key(pub [u8; KEYLENGTH]);

impl Key {
    /// Returns whether a key exists, or if it's just an all-0 placeholder.
    fn exists(&self) -> bool {
        return !self.0.iter().all(|&x| x == 0);
    }
}

impl From<Vec<u8>> for Key {
    fn from(v: Vec<u8>) -> Self {
        Self(
            v.try_into()
                .expect("vec key should fit in slice; check KEYLENGTH"),
        )
    }
}

impl From<[u8; KEYLENGTH]> for Key {
    fn from(v: [u8; KEYLENGTH]) -> Self {
        Self(v)
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&BASE64.encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        if s.is_empty() {
            return Ok(Self([0u8; KEYLENGTH]));
        }
        BASE64
            .decode(&s)
            .map_err(|_| serde::de::Error::custom(format!("Invalid base64 key {s:?}")))?
            .try_into()
            .map(Self)
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!("Expected {KEYLENGTH} bytes, got {}", v.len()))
            })
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.exists() {
            write!(f, "none")
        } else {
            write!(f, "{:?}", BASE64.encode(self.0))
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64.encode(self.0))
    }
}
