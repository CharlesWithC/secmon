use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::time::SystemTime;

#[derive(PartialEq)]
pub enum Mode {
    Server,
    Client,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Server => write!(f, "Mode::Server"),
            Mode::Client => write!(f, "Mode::Client"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    /// Name of user relevant to the session
    pub user: String,
    /// Remote origin of session (may be `None` for local session)
    pub from: Option<String>,
    /// Login time of session
    pub login: String,
}

pub type Sessions = Vec<Session>;
pub type SessionsResult = Result<Sessions, String>;

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

#[derive(Serialize, Deserialize)]
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

pub type WgPeers = Vec<WgPeer>;
pub type WgPeersResult = Result<WgPeers, String>;

pub struct Client {
    pub serial: u32,
    pub address: SocketAddr,
    pub sessions: SessionsResult,
    pub wg_peers: WgPeersResult,
    pub last_update: SystemTime,
}

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last_update_datetime: DateTime<Utc> = self.last_update.into();
        write!(
            f,
            "Client(serial={}, address=\"{}\", sessions[{}], wg_peers[{}], last_update=\"{}\")",
            self.serial,
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

#[derive(Serialize, Deserialize)]
/// Reprents a Command sent from server to client.
pub enum Command {
    /// Request a report of Session and WgPeer
    Report,

    /// Enable a systemctl service
    ///
    /// `bool` is whether to add '--now' flag
    ///
    /// `String` is the service name
    ServiceEnable(bool, String),

    /// Disable a systemctl service
    ///
    /// `bool` is whether to add '--now' flag
    ///
    /// `String` is the service name
    ServiceDisable(bool, String),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Report => write!(f, "Command::Report"),
            Command::ServiceEnable(now, service) => {
                write!(
                    f,
                    "Command::ServiceEnable(now={now}, service=\"{service}\")"
                )
            }
            Command::ServiceDisable(now, service) => {
                write!(
                    f,
                    "Command::ServiceDisable(now={now}, service=\"{service}\")"
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Represents a Message sent from client to server.
pub enum Message {
    /// Report of Session and WgPeer
    Report(SessionsResult, WgPeersResult),

    /// Generic result of a command
    ///
    /// `bool` is whether the command succeeded
    ///
    /// `String` is the message of the result
    Result(bool, String),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Report(sessions, wg_peers) => write!(
                f,
                "Message::Report(sessions[{}], wg_peers[{}])",
                sessions.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
                wg_peers.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
            ),
            Message::Result(success, message) => write!(
                f,
                "Message::Result(success={success}, message=\"{message}\")"
            ),
        }
    }
}

/// Default PORT that client and server agree
pub const PORT: u16 = 9992;
