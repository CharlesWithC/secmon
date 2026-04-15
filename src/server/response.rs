use std::time::SystemTime;

use crate::models::{ClientState, ErrUpdateClient, Response};

/// Process 'response' and update client information for client with given serial
pub fn handle_response(
    serial: u32,
    response: Response,
    mutex: &ClientState,
) -> Result<(), ErrUpdateClient> {
    let (_, clients) = &mut *mutex.lock().unwrap();
    if let Some(index) = clients.iter().position(|client| client.serial == serial) {
        match response {
            Response::Report(sessions, wg_peers) => {
                match &sessions {
                    Ok(sessions) => {
                        for session in sessions.iter() {
                            println!("{session}");
                        }
                    }
                    Err(error) => {
                        println!("Error: {error}");
                    }
                }
                match &wg_peers {
                    Ok(wg_peers) => {
                        for wg_peer in wg_peers.iter() {
                            println!("{wg_peer}");
                        }
                    }
                    Err(error) => {
                        println!("Error: {error}");
                    }
                }

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
