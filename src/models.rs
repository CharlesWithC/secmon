use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

pub mod hub;
pub mod node;
pub mod nodestate;
pub mod packet;

// DEFAULT VALUES
/// Default host for hub
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
/// Default port for hub and node
pub const DEFAULT_PORT: u16 = 9992;
/// Default socket directory for hub <=> cli
pub const DEFAULT_SOCKET_DIR: &str = "/var/run/secmon";

/// Launch arguments
pub enum LaunchArgs {
    Hub(IpAddr, u16),
    Node(IpAddr, u16, NodeConfig),
    Cli(String, Vec<String>),
}

impl fmt::Display for LaunchArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LaunchArgs::Hub(ip, port) => write!(f, "Hub(ip=\"{ip}\", port={port})"),
            LaunchArgs::Node(ip, port, node_config) => {
                write!(f, "Node(ip=\"{ip}\", port={port}, {node_config})")
            }
            LaunchArgs::Cli(command, args) => {
                write!(f, "Cli(command=\"{command}\", args=\"{}\")", args.join(" "))
            }
        }
    }
}

#[derive(Clone, Copy)]
/// Node configuration
pub struct NodeConfig {
    pub reconnect: bool,
    pub enable_sessions: bool,
    pub enable_wg_peers: bool,
}

impl fmt::Display for NodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "reconnect={}, sessions={}, wg_peers={}",
            self.reconnect, self.enable_sessions, self.enable_wg_peers
        )
    }
}
