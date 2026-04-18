use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

pub mod hub;
pub mod node;
pub mod packet;
pub mod nodestate;

// DEFAULT VALUES
/// Default host for hub.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
/// Default port for hub and node.
pub const DEFAULT_PORT: u16 = 9992;

#[derive(PartialEq)]
/// Launch mode
pub enum Mode {
    Hub,
    Node,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Hub => write!(f, "Mode::Hub"),
            Mode::Node => write!(f, "Mode::Node"),
        }
    }
}
