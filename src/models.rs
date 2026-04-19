use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

pub mod hub;
pub mod node;
pub mod nodestate;
pub mod packet;

// DEFAULT VALUES
/// Default host for hub.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
/// Default port for hub and node.
pub const DEFAULT_PORT: u16 = 9992;
/// Default socket directory for hub <=> cli.
pub const DEFAULT_SOCKET_DIR: &str = "/var/run/secmon";

/// Launch mode
pub enum Mode {
    Hub,
    Node(NodeConfig),
}

#[derive(Clone, Copy)]
pub struct NodeConfig {
    pub reconnect: bool,
    pub enable_sessions: bool,
    pub enable_wg_peers: bool,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Hub => write!(f, "Mode::Hub"),
            Mode::Node(node_config) => {
                write!(f, "Mode::Node({node_config})")
            }
        }
    }
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
