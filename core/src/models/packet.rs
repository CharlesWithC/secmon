use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::node::{NodeState, NodeUpdate};

/// Command sent from hub to node
#[derive(Serialize, Deserialize, Clone)]
pub enum Command {
    /// Request current node state
    NodeState,

    /// Execute a preconfigured allowed command
    Execute(Label, Stream),
}

pub type Label = String;
pub type Stream = bool;

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NodeState => write!(f, "Command::NodeState"),
            Self::Execute(label, stream) => {
                write!(f, "Command::Execute(label=\"{label}\", stream={stream})",)
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

    /// Partial result from streaming
    ResultStream(ResultStatus, Line),
}

/// Status of a streamed result
#[derive(Serialize, Deserialize, Clone)]
pub enum ResultStatus {
    Pending,
    Success,
    Failure,
}

impl fmt::Display for ResultStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
        }
    }
}

pub type Hostname = String;
pub type Success = bool;
pub type Message = String;
pub type Line = String;

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "Response::KeepAlive"),
            Self::Connect(hostname) => write!(f, "Response::Connect(hostname=\"{hostname}\")"),
            Self::NodeState(node_state) => write!(f, "Response::{node_state}",),
            Self::NodeUpdate(update) => write!(f, "Response::{update}",),
            Self::Result(success, message) => write!(
                f,
                "Response::Result(success={success}, message={:?})",
                message
            ),
            Self::ResultStream(status, line) => {
                write!(
                    f,
                    "Response::ResultStream(status=\"{status}\", line={:?})",
                    line
                )
            }
        }
    }
}
