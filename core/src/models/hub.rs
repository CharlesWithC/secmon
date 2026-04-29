use chrono::DateTime;
use chrono::offset::Utc;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::models::node::{NodeUpdate, Sessions, WgPeers};
use crate::models::packet::{Command, Response};
use crate::utils::get_display_len;

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
/// Represents a vector of subscribed clients.
///
/// Client adds a Sender here and retains the Receiver to receive updates.
///
/// If Sender errors (i.e. client disconnects), then the Sender is removed from the vector.
///
/// This is as if providing an email address to subscribe to a mail list.
pub type SubscribedClients = Vec<Sender<(u32, NodeUpdate)>>;
pub type HubState = (u32, HubNodes, SubscribedClients); // (counter, nodes, subscribers)
pub type HubStateMutex = Arc<Mutex<HubState>>;

/// Instance of a node
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    /// Serial number of node
    pub serial: u32,
    /// Socket address of node
    pub address: SocketAddr,
    /// Hostname of node
    pub hostname: String,
    /// User sessions collected by node
    pub sessions: Sessions,
    /// WireGuard peers collected by node
    pub wg_peers: WgPeers,
    /// Last state update received from node
    pub last_state_update: SystemTime,
    /// Whether node is connected (disconnected nodes are removed after grace period)
    pub connected: bool,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let last_update_dt: DateTime<Utc> = self.last_state_update.into();
        write!(
            f,
            "Node(serial={}, hostname=\"{}\", address=\"{}\", sessions[{}], wg_peers[{}], last_update=\"{}\")",
            self.serial,
            self.hostname,
            self.address,
            get_display_len(&self.sessions),
            get_display_len(&self.wg_peers),
            last_update_dt
        )
    }
}

/// Command sent from client to hub
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientCommand {
    /// Subscribe to node state updates
    ///
    /// Note: Client may not "unsubscribe". A separate
    /// connection should be used to send other commands.
    Subscribe,

    /// List all connected nodes
    List,

    /// Finds the first node matching address or hostname
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
            Self::Subscribe => write!(f, "ClientCommand::Subscribe"),
            Self::List => write!(f, "ClientCommand::List"),
            Self::FindNode(query) => {
                write!(f, "ClientCommand::FindNode(query=\"{query}\")")
            }
            Self::RawCommand(serial, command) => {
                write!(
                    f,
                    "ClientCommand::RawCommand(serial={serial}, command={command})"
                )
            }
            Self::Quit => write!(f, "ClientCommand::Quit"),
        }
    }
}

/// Result of processing a client command
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientResponse {
    /// Node state update, including tracked but not stored state
    NodeUpdate(u32, NodeUpdate),

    /// A list of all connected nodes
    List(Vec<Node>),

    /// A single node with its stored state
    Node(Node),

    /// Wraps a raw packet response
    RawResponse(Response),

    /// Generic failure with a string-based error
    Failure(ErrorMessage),
}

impl fmt::Display for ClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NodeUpdate(serial, update) => write!(
                f,
                "ClientResponse::NodeUpdate(serial={serial}, data={update})"
            ),
            Self::List(nodes) => {
                write!(f, "ClientResponse::List(nodes[{}])", nodes.len())
            }
            Self::Node(node) => write!(f, "ClientResponse::Node(node={node})"),
            Self::RawResponse(response) => {
                write!(f, "ClientResponse::RawResponse(response={response})")
            }
            Self::Failure(error) => {
                write!(f, "ClientResponse::Failure(error={:?})", error)
            }
        }
    }
}
