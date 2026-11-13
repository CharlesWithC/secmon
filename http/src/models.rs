use serde::Serialize;

use secmon::models::node::NodeUpdate;

/// Error Response
#[derive(Serialize)]
pub struct HttpError {
    pub error: String,
}

/// Struct for node data update with node serial
///
/// Core packet uses a tuple for this type; for serialization
/// purpose, we wrap it with a struct for http package.
#[derive(Serialize, Clone)]
pub struct BodyNodeUpdate {
    pub node_serial: u32,
    pub data: NodeUpdate,
}
