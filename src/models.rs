use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

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
    pub last_update: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Report,
    WgUp(String),
    WgDown(String),
    ServiceUp(String),
    ServiceDown(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Report(Vec<Session>, Vec<WgPeer>),
    Result(String),
}

pub const PORT: u16 = 9992;
