use std::time::SystemTime;

use crate::models::hub::{ErrHubState, HubState};
use crate::models::packet::Response;

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
