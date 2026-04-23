use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::time::SystemTime;

use crate::models::nodestate::{SessionsOpt, WgPeersOpt};
use crate::utils::get_display_len;

/// Instance of a node
#[derive(Serialize, Deserialize, Clone)]
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
    pub sessions: SessionsOpt,
    /// WireGuard peers collected by node
    pub wg_peers: WgPeersOpt,
    /// Last state update received from node
    pub last_state_update: SystemTime,
    /// Whether node is connected
    ///
    /// Note: When a node disconnects, there is a grace period of 30 seconds before it is removed.
    pub connected: bool,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last_update_dt: DateTime<Utc> = self.last_state_update.into();
        write!(
            f,
            "Node(serial={}, hostname=\"{}\", address=\"{}\", sessions[{}], wg_peers[{}], last_update=\"{}\")",
            self.serial,
            self.hostname,
            self.address,
            get_display_len(&self.sessions),
            get_display_len(&self.wg_peers),
            last_update_dt
        )
    }
}
