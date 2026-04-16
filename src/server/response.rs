use std::time::SystemTime;

use crate::models::{ClientState, ErrUpdateClient, Response};

/// Handles response from client.
pub fn handle_response(
    serial: u32,
    response: Response,
    client_state: &ClientState,
) -> Result<(), ErrUpdateClient> {
    let mut guard = client_state.lock().unwrap();
    let (_, ref mut clients) = *guard;
    if let Some(index) = clients.iter().position(|client| client.serial == serial) {
        match response {
            Response::Report(sessions, wg_peers) => {
                clients[index].sessions = sessions;
                clients[index].wg_peers = wg_peers;
                clients[index].last_update = SystemTime::now();
            }
            _ => {}
        }

        Ok(())
    } else {
        Err(ErrUpdateClient::SerialNotRecognized)
    }
}
