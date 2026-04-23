use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;

use crate::utils::get_display_len;

type ErrorMessage = String;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// Full state of a node
pub struct NodeState {
    pub sessions: SessionsOpt,
    pub wg_peers: WgPeersOpt,
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// User session collected by node
pub struct Session {
    /// Name of user relevant to the session
    pub user: String,
    /// Remote origin of session (may be `None` for local session)
    pub from: Option<String>,
    /// Login time of session
    pub login: SystemTime,
}

pub type Sessions = Result<Vec<Session>, ErrorMessage>;
pub type SessionsOpt = Option<Sessions>;

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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// WireGuard peer collected by node
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

pub type WgPeers = Result<Vec<WgPeer>, ErrorMessage>;
pub type WgPeersOpt = Option<WgPeers>;

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
