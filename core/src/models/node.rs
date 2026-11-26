use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::utils::get_vec_len;

/// Time in seconds between `KeepAlive` messages
pub const KEEPALIVE_INTERVAL: u64 = 60;

/// Error info for an attribute of node data
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
            Self::Initializing => write!(f, "Initializing"),
            Self::NotMonitored => write!(f, "NotMonitored"),
            Self::Message(message) => {
                write!(f, "Message(message={message:?})")
            }
        }
    }
}

/// Full tracked and stored state of a node, excluding state that is not stored
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
            get_vec_len(&self.sessions),
            get_vec_len(&self.wg_peers)
        )
    }
}

/// A connection-related event. No state/data update is included.
///
/// This event is emitted by hub's tcp connection manager.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeEvent {
    /// Node connected
    Connect(String),
    /// Node sent a `KeepAlive` message
    KeepAlive,
    /// Node disconnected
    Disconnect,
}

impl fmt::Display for NodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Connect(hostname) => write!(f, "Connect(hostname={hostname:?})"),
            Self::KeepAlive => write!(f, "KeepAlive"),
            Self::Disconnect => write!(f, "Disconnect"),
        }
    }
}

/// Atomic update from node on tracked state, including state that is not stored
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct NodeUpdate {
    pub sessions: Option<Sessions>,
    pub wg_peers: Option<WgPeers>,
    /// We use `AuthLogUpdate` because `AuthLog` does not embed errors.
    ///
    /// Stored states do not require such wrapping as they embed errors already.
    pub auth_log: Option<AuthLogUpdate>,
    /// Connection-related event emitted by hub.
    pub event: Option<NodeEvent>,
}

impl fmt::Display for NodeUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut data = Vec::<String>::new();
        if let Some(ref sessions) = self.sessions {
            data.push(format!("sessions[{}]", get_vec_len(sessions)));
        };
        if let Some(ref wg_peers) = self.wg_peers {
            data.push(format!("wg_peers[{}]", get_vec_len(wg_peers)));
        };
        if let Some(ref auth_log_res) = self.auth_log {
            match auth_log_res {
                Ok(auth_log) => data.push(format!("auth_log({auth_log})")),
                Err(e) => data.push(format!("auth_log(error={e})")),
            }
        };
        if let Some(ref event) = self.event {
            data.push(format!("event={event}"));
        }
        write!(f, "NodeUpdate({})", data.join(", "))
    }
}

/// User session (stored state)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
        write!(
            f,
            "Session(user={:?}, from={:?}, login={:?})",
            self.user,
            self.from.as_deref().unwrap_or("N/A"),
            DateTime::<Utc>::from(self.login).format("%F %T").to_string(),
        )
    }
}

/// WireGuard peer (stored state)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
        write!(
            f,
            "WgPeer(interface={:?}, peer={:?}, endpoint={:?}, latest_handshake={:?})",
            self.interface,
            self.peer,
            self.endpoint.as_deref().unwrap_or("N/A"),
            match self.latest_handshake {
                None => "N/A".to_owned(),
                Some(st) => DateTime::<Utc>::from(st).format("%F %T").to_string(),
            },
        )
    }
}

/// Auth log entry (tracked-only; not stored)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AuthLog {
    /// Time of the entry
    pub time: SystemTime,
    /// process relevant to the entry
    pub process: String,
    /// User relevant to entry
    pub user: String,
    /// Detail of the entry
    pub detail: AuthLogDetail,
}

pub type AuthLogUpdate = Result<AuthLog, NodeDataError>;

impl fmt::Display for AuthLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AuthLog(time={:?}, process={:?}, user={:?}, detail={})",
            DateTime::<Utc>::from(self.time).format("%F %T").to_string(),
            self.process,
            self.user,
            self.detail
        )
    }
}

/// Detail of auth log entry
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AuthLogDetail {
    /// SSH connection started
    ///
    /// Example:
    /// ```Accepted publickey for drako from 1.1.1.1 port 50000 ssh2: ...```
    SshConnect {
        host: String,
        port: u16,
        method: String,
    },
    /// SSH connection failed
    ///
    /// Example:
    /// ```Failed password for drako from 1.1.1.1.1 port 50000 ssh2```
    SshFailPassword { host: String, port: u16 },
    /// SSH connection closed
    ///
    /// Example:
    /// ```Disconnected from user drako 1.1.1.1 port 50000```
    SshDisconnect { host: String, port: u16 },
    /// SU session opened
    ///
    /// Example:
    /// ```pam_unix(_): session opened for user root(uid=0) by drako(uid=1000)```
    SuOpen { target_user: String },
    /// SU session failed
    ///
    /// Example:
    /// ```FAILED SU (to root) drako on pts/4```
    SuFail { target_user: String },
    /// SU session closed
    ///
    /// Note: The source/action user is not provided.
    ///
    /// Example:
    /// ```pam_unix(_): session closed for user root```
    SuClose { target_user: String },
}

impl fmt::Display for AuthLogDetail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SshConnect { host, port, method } => write!(
                f,
                "SshConnect(host={host:?}, port={port}, method={method:?})",
            ),
            Self::SshFailPassword { host, port } => {
                write!(f, "SshFailPassword(host={host:?}, port={port})")
            }
            Self::SshDisconnect { host, port } => {
                write!(f, "SshDisconnect(host={host:?}, port={port})")
            }
            Self::SuOpen { target_user } => {
                write!(f, "SuOpen(target_user={target_user:?})")
            }
            Self::SuFail { target_user } => {
                write!(f, "SuFail(target_user={target_user:?})")
            }
            Self::SuClose { target_user } => {
                write!(f, "SuClose(target_user={target_user:?})")
            }
        }
    }
}
