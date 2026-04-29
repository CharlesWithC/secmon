use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::utils::get_display_len;

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
                write!(f, "Message(message={:?})", message)
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
            get_display_len(&self.sessions),
            get_display_len(&self.wg_peers)
        )
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
        if let Some(ref auth_log_res) = self.auth_log {
            match auth_log_res {
                Ok(auth_log) => data.push(format!("auth_log({auth_log})")),
                Err(e) => data.push(format!("auth_log(error={e})")),
            }
        };
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
        let dt: DateTime<Utc> = self.time.into();
        let time_parsed = format!("{}", dt);

        write!(
            f,
            "AuthLog(time=\"{}\", process=\"{}\", user=\"{}\", detail={})",
            time_parsed, self.process, self.user, self.detail
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
    SshConnect(AuthOrigin, AuthLoginMethod),
    /// SSH connection failed
    ///
    /// Example:
    /// ```Failed password for drako from 1.1.1.1.1 port 50000 ssh2```
    SshFailPassword(AuthOrigin),
    /// SSH connection closed
    ///
    /// Example:
    /// ```Disconnected from user drako 1.1.1.1 port 50000```
    SshDisconnect(AuthOrigin),
    /// SU session opened
    ///
    /// Example:
    /// ```pam_unix(su:session): session opened for user root(uid=0) by drako(uid=0)```
    SuOpen(TargetUser),
    /// SU session failed
    ///
    /// Example:
    /// ```FAILED SU (to root) drako on pts/4```
    SuFail(TargetUser),
    /// SU session closed
    /// 
    /// Note: The source/action user is not provided.
    ///
    /// Example:
    /// ```pam_unix(su:session): session closed for user root```
    SuClose(TargetUser),
}

impl fmt::Display for AuthLogDetail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SshConnect((host, port), login_method) => write!(
                f,
                "SshConnect(host=\"{}\", port={}, method=\"{}\")",
                host, port, login_method
            ),
            Self::SshFailPassword((host, port)) => {
                write!(f, "SshFailPassword(host=\"{}\", port={})", host, port)
            }
            Self::SshDisconnect((host, port)) => {
                write!(f, "SshDisconnect(host=\"{}\", port={})", host, port)
            }
            Self::SuOpen(target) => {
                write!(f, "SuOpen(target=\"{}\")", target)
            }
            Self::SuFail(target) => {
                write!(f, "SuFail(target=\"{}\")", target)
            }
            Self::SuClose(target) => {
                write!(f, "SuClose(target=\"{}\")", target)
            }
        }
    }
}

/// SSH Origin
pub type AuthOrigin = (String, u16);
/// SSH Login Method (publickey, password)
pub type AuthLoginMethod = String;
/// SU Target User
pub type TargetUser = String;
