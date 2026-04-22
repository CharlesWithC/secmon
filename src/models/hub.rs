use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::models::node::Node;
use crate::models::packet::{Command, Response};

/// Represents data sent in internal channel between command and node handler.
///
/// The `Sender<Response>` is a one-time channel for node handler to return response to.
///
/// This is as if sending a mail with a return envelop attached.
pub type ChannelPacket = (Command, Sender<Response>);
/// Represents a vector of connected nodes.
/// 
/// The `Sender<ChannelPacket>` is a long-living channel to send local commands to.
/// 
/// This is as if telling someone your mail carrier's name who would deliver mails to you.
pub type HubNodes = Vec<(Node, Sender<ChannelPacket>)>;
pub type HubState = (u32, HubNodes); // (counter, nodes)
pub type HubStateMutex = Arc<Mutex<HubState>>;

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
