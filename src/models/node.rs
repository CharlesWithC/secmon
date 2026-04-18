use chrono::DateTime;
use chrono::offset::Utc;
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::models::nodestate::{SessionsResult, WgPeersResult};

/// Instance of a node
pub struct Node {
    /// Serial number of node
    ///
    /// Each time a new node connects, the serial number should increase.
    pub serial: u32,
    /// Socket address of node
    pub address: SocketAddr,
    /// Hostname of node
    pub hostname: String,
    /// User sessions collected by node
    pub sessions: SessionsResult,
    /// WireGuard peers collected by node
    pub wg_peers: WgPeersResult,
    /// Last state update received from node
    pub last_state_update: UpdateTime,
}

pub type NodeState = Arc<Mutex<(SessionsResult, WgPeersResult, UpdateTime)>>;
pub type UpdateTime = SystemTime;

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last_update_datetime: DateTime<Utc> = self.last_state_update.into();
        write!(
            f,
            "Node(serial={}, hostname=\"{}\", address=\"{}\", sessions[{}], wg_peers[{}], last_update=\"{}\")",
            self.serial,
            self.hostname,
            self.address,
            self.sessions
                .as_ref()
                .map(|v| v.len() as isize)
                .unwrap_or(-1),
            self.wg_peers
                .as_ref()
                .map(|v| v.len() as isize)
                .unwrap_or(-1),
            last_update_datetime.format("%F %T")
        )
    }
}
