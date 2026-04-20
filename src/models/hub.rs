use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::models::node::Node;

pub type HubState = Arc<Mutex<(u32, Vec<Node>)>>;

/// Error when updating hub state
pub enum ErrHubState {
    /// Node cannot be recognized based on serial
    SerialNotRecognized,
}

impl fmt::Display for ErrHubState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrHubState::SerialNotRecognized => write!(f, "SerialNotRecognized"),
        }
    }
}

/// Represents a command line control command.
#[derive(Serialize, Deserialize)]
pub enum CtrlCmd {
    /// List all connected nodes
    List,
}

impl fmt::Display for CtrlCmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CtrlCmd::List => write!(f, "CtrlCmd::List"),
        }
    }
}

/// Represnts the result of executing a control command.
#[derive(Serialize, Deserialize)]
pub enum CtrlRes {
    /// A list of all connected nodes
    List(Vec<Node>),
}

impl fmt::Display for CtrlRes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CtrlRes::List(nodes) => write!(f, "CtrlRes::List(nodes[{}])", nodes.len()),
        }
    }
}
