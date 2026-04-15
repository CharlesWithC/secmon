use anyhow::Result;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// DEFAULT VALUES
/// Default host for server.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
/// Default port for server and client.
pub const DEFAULT_PORT: u16 = 9992;

pub type ErrorMessage = String;

#[derive(PartialEq)]
/// Launch mode
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// User session collected by client
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
/// WireGuard peer collected by client
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

/// Instance of client that server maintains
pub struct Client {
    /// Serial number of client
    ///
    /// Each time a new client connects, the serial number should increase.
    pub serial: u32,
    /// Socket address of client
    pub address: SocketAddr,
    /// Hostname of client
    pub hostname: String,
    /// User sessions collected by client
    pub sessions: SessionsResult,
    /// WireGuard peers collected by client
    pub wg_peers: WgPeersResult,
    /// Last update received from client
    ///
    /// Note: This is only updated when a new report is received
    pub last_update: SystemTime,
}

pub type ClientState = Arc<Mutex<(u32, Vec<Client>)>>;

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last_update_datetime: DateTime<Utc> = self.last_update.into();
        write!(
            f,
            "Client(serial={}, hostname=\"{}\", address=\"{}\", sessions[{}], wg_peers[{}], last_update=\"{}\")",
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

/// Error on updating client information
pub enum ErrUpdateClient {
    /// Client cannot be recognized based on serial
    SerialNotRecognized,
}

impl fmt::Display for ErrUpdateClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrUpdateClient::SerialNotRecognized => write!(f, "SerialNotRecognized"),
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Reprents a Command sent from server to client.
pub enum Command {
    /// Request a `KeepAlive` response from client
    KeepAlive,

    /// Request a single report of Session and WgPeer
    Report,

    /// Request client to sync report updates until stopped
    ReportSyncStart,

    /// Request client to stop syncing report updates
    ReportSyncStop,

    /// Enable a systemctl service
    ServiceEnable(FlagNow, ServiceName),

    /// Disable a systemctl service
    ServiceDisable(FlagNow, ServiceName),

    /// Reboot client server at a specific time
    Reboot(SystemTime),
}

pub type FlagNow = bool;
pub type ServiceName = String;

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::KeepAlive => write!(f, "Command::KeepAlive"),
            Command::Report => write!(f, "Command::Report"),
            Command::ReportSyncStart => write!(f, "Command::ReportSyncStart"),
            Command::ReportSyncStop => write!(f, "Command::ReportSyncStop"),
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
            Command::Reboot(time) => {
                let time: DateTime<Utc> = (*time).into();
                write!(f, "Command::Reboot(time=\"{}\")", time.format("%F %T"))
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Represents a Message sent from client to server.
pub enum Response {
    /// `KeepAlive` acknowledgement
    KeepAlive,

    /// Connection successful
    ///
    /// This response is only sent once on connection establishment
    Connect(Hostname),

    /// Report of Session and WgPeer
    Report(SessionsResult, WgPeersResult),

    /// Report sync status
    ///
    /// This response is used on `ReportSyncStart` and `ReportSyncStop` commands
    ReportSync(Enabled),

    /// Generic result of a command
    Result(Success, Message),
}

pub type Hostname = String;
pub type Enabled = bool;
pub type Success = bool;
pub type Message = String;

pub type ReportState = Arc<Mutex<(SessionsResult, WgPeersResult, UpdateTime)>>;
pub type UpdateTime = SystemTime;

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Response::KeepAlive => write!(f, "Response::KeepAlive"),
            Response::Connect(hostname) => write!(f, "Response::Connect(hostname=\"{hostname}\")"),
            Response::Report(sessions, wg_peers) => write!(
                f,
                "Response::Report(sessions[{}], wg_peers[{}])",
                sessions.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
                wg_peers.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
            ),
            Response::ReportSync(enabled) => write!(f, "Response::ReportSync(enabled={enabled})"),
            Response::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message=\"{message}\")"
            ),
        }
    }
}
