use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};

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
