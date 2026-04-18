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
