use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::node::{NodeState, NodeUpdate};

/// Command sent from hub to node
#[derive(Serialize, Deserialize, Clone)]
pub enum Command {
    /// Request current node state
    NodeState,

    /// Execute a preconfigured allowed command
    Execute(Label),
}

pub type Label = String;

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::NodeState => write!(f, "Command::NodeState"),
            Command::Execute(label) => {
                write!(f, "Command::Execute(label=\"{label}\")",)
            }
        }
    }
}

/// Response sent from node to hub
#[derive(Serialize, Deserialize, Clone)]
pub enum Response {
    /// `KeepAlive` acknowledgement
    KeepAlive,

    /// Connection successful
    ///
    /// This response is only sent once on connection establishment.
    Connect(Hostname),

    /// Full node state of all stored data
    ///
    /// Note: This may only be requested by a `Command`.
    NodeState(NodeState),

    /// Atomic update of node state, including tracked but not stored data
    ///
    /// Note: This is sent automatically and may not be requested manually.
    NodeUpdate(NodeUpdate),

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
            Response::NodeUpdate(update) => write!(f, "Response::{update}",),
            Response::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message={:?})",
                message
            ),
        }
    }
}
