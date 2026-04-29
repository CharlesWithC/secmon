use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::node::{NodeState, NodeUpdate};

/// Command sent from hub to node
#[derive(Serialize, Deserialize, Clone)]
pub enum Command {
    /// Request current node state
    NodeState,

    /// Execute a preconfigured allowed command
    ///
    /// If `Stream=false`, then a single `Result` will be responded when command completes.
    ///
    /// If `Stream=true`, then `ResultStream` will be responded for each line of output, until
    /// command completes and a non-pending status with empty line output is returned.
    ///
    /// `REMOTE_EXEC_TIMEOUT` applies to timeout waiting for a response - that is, for streamed
    /// response, timeout will not occur as long as something is responded within timeout period.
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

    /// Full result of executing a command
    ///
    /// Note: The result is only returned when the command completes.
    Result(Success, Message),

    /// Partial result of executing a command
    ///
    /// Note: `ResultStatus` is `Pending` until the command completes, where
    /// an empty `Line` along with either `Success` or `Failure` is returned.
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
