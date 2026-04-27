use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::utils::get_display_len;

/// Error info for an attribute of node data
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeDataError {
    /// Hub has not received the first update from node
    Initializing,

    /// Node is not monitoring the specific attribute
    NotMonitored,

    /// Generic error message on collecting information
    Message(String),
}

impl fmt::Display for NodeDataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeDataError::Initializing => write!(f, "NodeDataError::Initializing"),
            NodeDataError::NotMonitored => write!(f, "NodeDataError::NotMonitored"),
            NodeDataError::Message(message) => {
                write!(f, "NodeDataError::Message(message={:?})", message)
            }
        }
    }
}

/// Full tracked and stored state of a node, excluding state that is not stored
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct NodeState {
    pub sessions: Sessions,
    pub wg_peers: WgPeers,
}

pub type NodeStateMutex = Arc<Mutex<NodeState>>;

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NodeState(sessions[{}], wg_peers[{}])",
            get_display_len(&self.sessions),
            get_display_len(&self.wg_peers)
        )
    }
}

/// Atomic update from node on tracked state, including state that is not stored
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct NodeUpdate {
    pub sessions: Option<Sessions>,
    pub wg_peers: Option<WgPeers>,
}

impl fmt::Display for NodeUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut data = Vec::<String>::new();
        if let Some(ref sessions) = self.sessions {
            data.push(format!("sessions[{}]", get_display_len(sessions)));
        };
        if let Some(ref wg_peers) = self.wg_peers {
            data.push(format!("wg_peers[{}]", get_display_len(wg_peers)));
        };
        write!(f, "NodeUpdate({})", data.join(", "))
    }
}

/// User session
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Session {
    /// Name of user relevant to the session
    pub user: String,
    /// Remote origin of session (may be `None` for local session)
    pub from: Option<String>,
    /// Login time of session
    pub login: SystemTime,
}

pub type Sessions = Result<Vec<Session>, NodeDataError>;

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dt: DateTime<Utc> = self.login.into();
        let login_parsed = format!("{}", dt);

        write!(
            f,
            "Session(user=\"{}\", from=\"{}\", login=\"{}\")",
            self.user,
            self.from.as_deref().unwrap_or("N/A"),
            login_parsed
        )
    }
}

/// WireGuard peer
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct WgPeer {
    /// WireGuard interface
    pub interface: String,
    /// WireGuard peer
    pub peer: String,
    /// WireGuard peer endpoint (connecting IP/port)
    pub endpoint: Option<String>,
    /// WireGuard peer last handshake (last connection time)
    pub latest_handshake: Option<SystemTime>,
}

pub type WgPeers = Result<Vec<WgPeer>, NodeDataError>;

impl fmt::Display for WgPeer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut latest_handshake_parsed = String::from("N/A");
        if let Some(st) = self.latest_handshake {
            let dt: DateTime<Utc> = st.into();
            latest_handshake_parsed = format!("{}", dt);
        }

        write!(
            f,
            "WgPeer(interface=\"{}\", peer=\"{}\", endpoint=\"{}\", latest_handshake=\"{}\")",
            self.interface,
            self.peer,
            self.endpoint.as_deref().unwrap_or("N/A"),
            latest_handshake_parsed
        )
    }
}
