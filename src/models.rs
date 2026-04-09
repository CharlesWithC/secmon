use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::SystemTime;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Server,
    Client,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub user: String,
    pub from: IpAddr,
    pub login: String,
    pub what: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WgPeer {
    pub interface: String,
    pub peer: String,
    pub endpoint: SocketAddr,
    pub latest_handshake: String,
}

#[derive(Debug)]
pub struct Client {
    pub serial: u32,
    pub address: SocketAddr,
    pub sessions: Vec<Session>,
    pub wg_peers: Vec<WgPeer>,
    pub last_update: SystemTime,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
/// Represents a Message sent from client to server.
pub enum Message {
    /// Report of Session and WgPeer
    Report(Vec<Session>, Vec<WgPeer>),

    /// Generic result of a command
    /// 
    /// `bool` is whether the command succeeded
    /// 
    /// `String` is the message of the result
    Result(bool, String),
}

/// Default PORT that client and server agree
pub const PORT: u16 = 9992;
