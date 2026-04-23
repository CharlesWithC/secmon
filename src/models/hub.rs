use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::models::node::Node;
use crate::models::packet::{Command, Response};

type ErrorMessage = String;

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

/// Command sent from end-user client to hub
#[derive(Serialize, Deserialize)]
pub enum ClientCommand {
    /// List all connected nodes
    List,

    /// Finds the first node matching some identifier
    ///
    /// Hub will try to match the query string with IP and hostname
    FindNode(String),

    /// Wraps a raw packet command
    RawCommand(Serial, Command),

    /// Close connection
    Quit,
}

/// Serial number of a node
pub type Serial = u32;

impl fmt::Display for ClientCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientCommand::List => write!(f, "ClientCommand::List"),
            ClientCommand::FindNode(query) => write!(f, "ClientCommand::FindNode(query=\"{query}\")"),
            ClientCommand::RawCommand(serial, command) => {
                write!(f, "ClientCommand::RawCommand(serial={serial}, command={command})")
            }
            ClientCommand::Quit => write!(f, "ClientCommand::Quit"),
        }
    }
}

/// Result of executing an end-user client command
#[derive(Serialize, Deserialize)]
pub enum ClientResponse {
    /// A list of all connected nodes
    List(Vec<Node>),

    /// A single node
    Node(Node),

    /// Wraps a raw packet response
    RawResponse(Response),

    /// A generic failure with a string-based error
    Failure(ErrorMessage),
}

impl fmt::Display for ClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientResponse::List(nodes) => write!(f, "ClientResponse::List(nodes[{}])", nodes.len()),
            ClientResponse::Node(node) => write!(f, "ClientResponse::Node(node={node})"),
            ClientResponse::RawResponse(response) => {
                write!(f, "ClientResponse::RawResponse(response={response})")
            }
            ClientResponse::Failure(error) => {
                write!(f, "ClientResponse::Failure(error=\"{error}\")")
            }
        }
    }
}
