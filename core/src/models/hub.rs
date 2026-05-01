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

/// Represents command sent in channel between local command and remote node handler.
///
/// `Sender<Response>` is a one-time channel for node handler to return response to.
///
/// `SystemTime` decides past what time should a channel packet be ignored.
///
/// This is as if sending a mail with a return envelop attached.
pub type ChannelPacket = (Command, Sender<Response>, SystemTime);
/// Represents a vector of connected nodes.
///
/// `Sender<ChannelPacket>` is a long-living channel to send local commands to.
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
    /// Note: Client may not "unsubscribe" without closing connection.
    /// A separate connection should be used to send other commands.
    Subscribe,

    /// List all recently connected nodes
    ListNodes,

    /// Finds the first node matching serial, address or hostname
    FindNode { query: String },

    /// Raw command to be directly relayed to node
    RawCommand {
        node_serial: u32,
        command: Command,
        expire_time: SystemTime,
    },
}

impl fmt::Display for ClientCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Subscribe => write!(f, "ClientCommand::Subscribe"),
            Self::ListNodes => write!(f, "ClientCommand::ListNodes"),
            Self::FindNode { query } => {
                write!(f, "ClientCommand::FindNode(query=\"{query}\")")
            }
            Self::RawCommand {
                node_serial,
                command,
                expire_time,
            } => {
                let expire_time_dt: DateTime<Utc> = (*expire_time).into();
                write!(
                    f,
                    "ClientCommand::RawCommand(node_serial={node_serial}, command={command}, expire_time=\"{expire_time_dt}\")"
                )
            }
        }
    }
}

/// Result of processing a client command
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientResponse {
    /// Node state update, including tracked but not stored state
    NodeUpdate { node_serial: u32, data: NodeUpdate },

    /// All recently connected nodes
    Nodes(Vec<Node>),

    /// Single recently connected node
    Node(Node),

    /// Raw response directly relayed from node
    RawResponse(Response),

    /// Generic error
    Error(String),
}

impl fmt::Display for ClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NodeUpdate { node_serial, data } => write!(
                f,
                "ClientResponse::NodeUpdate(node_serial={node_serial}, data={data})"
            ),
            Self::Nodes(nodes) => {
                write!(f, "ClientResponse::Nodes(len={})", nodes.len())
            }
            Self::Node(node) => write!(f, "ClientResponse::{node}"),
            Self::RawResponse(response) => {
                write!(f, "ClientResponse::RawResponse(response={response})")
            }
            Self::Error(error) => {
                write!(f, "ClientResponse::Error(error={:?})", error)
            }
        }
    }
}
