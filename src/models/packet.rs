use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::nodestate::{NodeState, NodeStateDiff};

/// Command sent from hub to node
#[derive(Serialize, Deserialize, Clone)]
pub enum Command {
    /// Request current node state
    NodeState,

    /// Run a preconfigured allowed command
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
    /// This response is only sent once on connection establishment
    Connect(Hostname),

    /// Full node state
    ///
    /// Note: This should only be requested by a `Command`.
    ///
    /// The full node state is not used to update hub state,
    /// as hub state solely relies on diff rather than full state.
    NodeState(NodeState),

    /// Difference of node state compared with last update
    ///
    /// Note: This cannot be requested by a `Command` because
    /// node sends diff updates automatically.
    ///
    /// That said, when handling `Response`, `NodeStateDiff` can be safely
    /// ignored, in terms of relaying `Response` to command handler.
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
                "Response::Result(success={success}, message={:?})",
                message
            ),
        }
    }
}
