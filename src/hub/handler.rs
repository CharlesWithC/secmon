use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::hub::{ErrHubState, HubState};
use crate::models::node::Node;
use crate::models::packet::Response;

/// Initializes node connection.
///
/// Returns `serial` of the node for later identification.
pub fn handle_new_node(address: &SocketAddr, hostname: &String, hub_state: &HubState) -> u32 {
    let mut guard = hub_state.lock().unwrap();
    let (ref mut counter, ref mut nodes) = *guard;

    // increment counter for nodes
    *counter += 1;

    // initialize node, add node to vector
    let node = Node {
        serial: *counter,
        address: address.clone(),
        hostname: hostname.clone(),
        sessions: Err("Initializing".to_owned()),
        wg_peers: Err("Initializing".to_owned()),
        last_state_update: UNIX_EPOCH,
    };
    nodes.push(node);

    // return node serial
    *counter
}

/// Handles response from node.
pub fn handle_response(
    serial: u32,
    response: Response,
    hub_state: &HubState,
) -> Result<(), ErrHubState> {
    let mut guard = hub_state.lock().unwrap();
    let (_, ref mut nodes) = *guard;
    if let Some(index) = nodes.iter().position(|node| node.serial == serial) {
        match response {
            Response::NodeState(sessions, wg_peers) => {
                nodes[index].sessions = sessions;
                nodes[index].wg_peers = wg_peers;
                nodes[index].last_state_update = SystemTime::now();
            }
            _ => {}
        }

        Ok(())
    } else {
        Err(ErrHubState::SerialNotRecognized)
    }
}
