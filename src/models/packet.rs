use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::nodestate::{NodeState, NodeStateDiff};

/// Whether to enable or disable a service
#[derive(Serialize, Deserialize)]
pub enum ServiceMode {
    Enable,
    Disable,
}

impl fmt::Display for ServiceMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceMode::Enable => write!(f, "Enable"),
            ServiceMode::Disable => write!(f, "Disable"),
        }
    }
}

/// Command sent from hub to node
#[derive(Serialize, Deserialize)]
pub enum Command {
    /// Request current node state
    NodeState,

    /// Manage some systemctl services
    Service(ServiceMode, FlagNow, Vec<ServiceName>),

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
            Command::Service(mode, now, services) => {
                write!(
                    f,
                    "Command::ServiceEnable(mode=\"{mode}\", now={now}, services=[\"{}\"])",
                    services.join("\", \"")
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

/// Response sent from node to hub
#[derive(Serialize, Deserialize)]
pub enum Response {
    /// `KeepAlive` acknowledgement
    KeepAlive,

    /// Connection successful
    ///
    /// This response is only sent once on connection establishment
    Connect(Hostname),

    /// Full node state of Session and WgPeer
    NodeState(NodeState),

    /// Difference of node state compared with last update
    NodeStateDiff(NodeStateDiff),

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
            Response::NodeState(node_state) => write!(f, "Response::{node_state}",),
            Response::NodeStateDiff(diff) => write!(f, "Response::{diff}",),
            Response::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message=\"{message}\")"
            ),
        }
    }
}
