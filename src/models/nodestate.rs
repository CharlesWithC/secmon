use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;

use crate::utils::get_display_len;

/// Error info for an attribute of node state
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeStateError {
    /// Hub has not received the first update from node
    Initializing,

    /// Node is not monitoring the specific attribute
    NotMonitored,

    /// Generic error message on collecting information
    Message(String),
}

impl fmt::Display for NodeStateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeStateError::Initializing => write!(f, "NodeStateError::Initializing"),
            NodeStateError::NotMonitored => write!(f, "NodeStateError::NotMonitored"),
            NodeStateError::Message(message) => {
                write!(f, "NodeStateError::Message(message={:?})", message)
            }
        }
    }
}

/// Full state of a node
///
/// A `None` attribute means that the attribute is not monitored.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct NodeState {
    pub sessions: Sessions,
    pub wg_peers: WgPeers,
}

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

/// The difference of a node state
///
/// A `None` attribute means the attribute is not updated.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct NodeStateDiff {
    pub sessions: Option<Sessions>,
    pub wg_peers: Option<WgPeers>,
}

impl fmt::Display for NodeStateDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut diff = Vec::<String>::new();
        if let Some(ref sessions) = self.sessions {
            diff.push(format!("sessions[{}]", get_display_len(sessions)));
        };
        if let Some(ref wg_peers) = self.wg_peers {
            diff.push(format!("wg_peers[{}]", get_display_len(wg_peers)));
        };
        write!(f, "NodeStateDiff({})", diff.join(", "))
    }
}

/// User session collected by node
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Session {
    /// Name of user relevant to the session
    pub user: String,
    /// Remote origin of session (may be `None` for local session)
    pub from: Option<String>,
    /// Login time of session
    pub login: SystemTime,
}

pub type Sessions = Result<Vec<Session>, NodeStateError>;

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

/// WireGuard peer collected by node
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

pub type WgPeers = Result<Vec<WgPeer>, NodeStateError>;

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
