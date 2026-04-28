use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

pub mod hub;
pub mod node;
pub mod packet;

// DEFAULT VALUES
/// Default host for hub
pub const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
/// Default port for hub and node
pub const DEFAULT_PORT: u16 = 9992;

/// Launch arguments
pub enum LaunchArgs {
    Hub(IpAddr, u16, HubConfig),
    Node(IpAddr, u16, NodeConfig),
    Client(String),
}

impl fmt::Display for LaunchArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hub(ip, port, hub_config) => {
                write!(f, "Hub(ip=\"{ip}\", port={port}, {hub_config})")
            }
            Self::Node(ip, port, node_config) => {
                write!(f, "Node(ip=\"{ip}\", port={port}, {node_config})")
            }
            Self::Client(command) => {
                write!(f, "Client(command=\"{}\")", command)
            }
        }
    }
}

// Hub configuration
#[derive(Clone, Copy)]
pub struct HubConfig {
    pub remote_exec_timeout: u64,
    pub disconnect_grace_period: u64,
    pub assume_hostname_unique: bool,
}

impl fmt::Display for HubConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "disconnect_grace_period={}, assume_hostname_unique={}",
            self.disconnect_grace_period, self.assume_hostname_unique
        )
    }
}

/// Node configuration
#[derive(Clone, Copy)]
pub struct NodeConfig {
    pub reconnect: bool,
    pub enable_sessions: bool,
    pub enable_wg_peers: bool,
    pub enable_auth_log: bool,
}

impl fmt::Display for NodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "reconnect={}, sessions={}, wg_peers={}, auth_log={}",
            self.reconnect, self.enable_sessions, self.enable_wg_peers, self.enable_auth_log
        )
    }
}
