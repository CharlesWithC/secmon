use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::nodestate::{SessionsResult, WgPeersResult};

#[derive(Serialize, Deserialize)]
/// Reprents a Command sent from hub to node.
pub enum Command {
    /// Request a `KeepAlive` response from node
    KeepAlive,

    /// Request current node state
    NodeState,

    /// Request node to sync state updates until stopped
    StateSyncStart,

    /// Request node to stop syncing state updates
    StateSyncStop,

    /// Enable a systemctl service
    ServiceEnable(FlagNow, ServiceName),

    /// Disable a systemctl service
    ServiceDisable(FlagNow, ServiceName),

    /// Schedule node server reboot
    Reboot(Minutes),

    /// Cancel node server reboot schedule
    RebootCancel,
}

pub type FlagNow = bool;
pub type ServiceName = String;
pub type Minutes = u32;

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::KeepAlive => write!(f, "Command::KeepAlive"),
            Command::NodeState => write!(f, "Command::NodeState"),
            Command::StateSyncStart => write!(f, "Command::StateSyncStart"),
            Command::StateSyncStop => write!(f, "Command::StateSyncStop"),
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
            Command::RebootCancel => {
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

    /// Node state sync status
    ///
    /// This response in response to `StateSyncStart` and `StateSyncStop` commands
    StateSync(Enabled),

    /// Generic result of a command
    Result(Success, Message),
}

pub type Hostname = String;
pub type Enabled = bool;
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
            Response::StateSync(enabled) => write!(f, "Response::StateSync(enabled={enabled})"),
            Response::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message=\"{message}\")"
            ),
        }
    }
}
