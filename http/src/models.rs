use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};

use secmon::models::node::NodeUpdate;

// DEFAULT VALUES
/// Default host for hub
pub const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
/// Default port for hub and node
pub const DEFAULT_PORT: u16 = 9993;

/// Error Response
#[derive(Serialize)]
pub struct HttpError {
    pub error: String,
}

/// Struct for node data update with node serial
///
/// Core packet uses a tuple for this type; for serialization
/// purpose,we wrap it with a struct in http package.
#[derive(Serialize, Clone)]
pub struct BodyNodeUpdate {
    pub node_serial: u32,
    pub data: NodeUpdate,
}
