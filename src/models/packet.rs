use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::nodestate::{SessionsResult, WgPeersResult};

#[derive(Serialize, Deserialize)]
/// Reprents a Command sent from hub to node.
pub enum Command {
    /// Request current node state
    NodeState,

    /// Enable a systemctl service
    ServiceEnable(FlagNow, ServiceName),

    /// Disable a systemctl service
    ServiceDisable(FlagNow, ServiceName),

    /// Schedule node server reboot
    Reboot(Minutes),

    /// Cancel node server reboot schedule
    ShutdownCancel,
}

pub type FlagNow = bool;
pub type ServiceName = String;
pub type Minutes = u32;

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::NodeState => write!(f, "Command::NodeState"),
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
            Command::Reboot(minutes) => {
                write!(f, "Command::Reboot(in=\"{}min\")", minutes)
            }
            Command::ShutdownCancel => {
                write!(f, "Command::RebootCancel")
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Represents a Response sent from node to hub.
pub enum Response {
    /// `KeepAlive` acknowledgement
    KeepAlive,

    /// Connection successful
    ///
    /// This response is only sent once on connection establishment
    Connect(Hostname),

    /// Node state of Session and WgPeer
    NodeState(SessionsResult, WgPeersResult),

    /// Generic result of a command
    Result(Success, Message),
}

pub type Hostname = String;
pub type Success = bool;
pub type Message = String;

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Response::KeepAlive => write!(f, "Response::KeepAlive"),
            Response::Connect(hostname) => write!(f, "Response::Connect(hostname=\"{hostname}\")"),
            Response::NodeState(sessions, wg_peers) => write!(
                f,
                "Response::NodeState(sessions[{}], wg_peers[{}])",
                sessions.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
                wg_peers.as_ref().map(|v| v.len() as isize).unwrap_or(-1),
            ),
            Response::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message=\"{message}\")"
            ),
        }
    }
}
