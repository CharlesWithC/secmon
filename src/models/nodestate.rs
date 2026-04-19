use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

type ErrorMessage = String;

pub type NodeState = (SessionsResult, WgPeersResult);

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// User session collected by node
pub struct Session {
    /// Name of user relevant to the session
    pub user: String,
    /// Remote origin of session (may be `None` for local session)
    pub from: Option<String>,
    /// Login time of session
    pub login: String,
}

pub type Sessions = Vec<Session>;
pub type SessionsResult = Result<Sessions, ErrorMessage>;

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Session(user=\"{}\", from=\"{}\", login=\"{}\")",
            self.user,
            self.from.as_deref().unwrap_or("N/A"),
            self.login
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
    pub latest_handshake: Option<String>,
}

pub type WgPeers = Vec<WgPeer>;
pub type WgPeersResult = Result<WgPeers, ErrorMessage>;

impl fmt::Display for WgPeer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "WgPeer(interface=\"{}\", peer=\"{}\", endpoint=\"{}\", latest_handshake=\"{}\")",
            self.interface,
            self.peer,
            self.endpoint.as_deref().unwrap_or("N/A"),
            self.latest_handshake.as_deref().unwrap_or("N/A")
        )
    }
}
