use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::node::{NodeState, NodeUpdate};

/// Command sent from hub to node
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    /// Request current node state
    ///
    /// Note: Technically there is no reason to use this command, since hub stores the
    /// state and client should simply request the data from hub.
    NodeState,

    /// Execute a preconfigured allowed command
    ///
    /// If `stream=false`, then a single `Result` will be responded when command completes.
    ///
    /// If `stream=true`, then `ResultStream` will be responded for each line of output, until
    /// command completes and a non-pending status with empty line data is returned.
    Execute { command_label: String, stream: bool },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NodeState => write!(f, "Command::NodeState"),
            Self::Execute {
                command_label,
                stream,
            } => {
                write!(
                    f,
                    "Command::Execute(command_label=\"{command_label}\", stream={stream})",
                )
            }
        }
    }
}

/// Response sent from node to hub
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
    /// `KeepAlive` acknowledgement
    KeepAlive,

    /// Connection successful
    ///
    /// This response is only sent once on connection establishment.
    Connect { hostname: String },

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
    /// `ResultStatus` should not be `Pending` as streaming is not enabled.
    Result {
        status: ResultStatus,
        output: String,
    },

    /// Partial result of executing a command
    ///
    /// Note: `ResultStatus` is `Pending` until the command completes, where
    /// an empty `Line` along with either `Success` or `Failure` is returned.
    ResultStream { status: ResultStatus, line: String },
}

/// Status of a streamed result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ResultStatus {
    Pending,
    Timeout,
    Success,
    Failure,
}

impl fmt::Display for ResultStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Timeout => write!(f, "timeout"),
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
        }
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "Response::KeepAlive"),
            Self::Connect { hostname } => write!(f, "Response::Connect(hostname=\"{hostname}\")"),
            Self::NodeState(node_state) => write!(f, "Response::{node_state}",),
            Self::NodeUpdate(update) => write!(f, "Response::{update}",),
            Self::Result { status, output } => write!(
                f,
                "Response::Result(status=\"{status}\", output={:?})",
                output
            ),
            Self::ResultStream { status, line } => {
                write!(
                    f,
                    "Response::ResultStream(status=\"{status}\", line={:?})",
                    line
                )
            }
        }
    }
}
