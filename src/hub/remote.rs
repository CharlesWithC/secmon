use anyhow::Result;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::iosered::IOSerialized;
use crate::models::hub::{ErrHubState, HubState};
use crate::models::node::Node;
use crate::models::packet::Response;

/// Initializes node connection.
///
/// Returns `serial` of the node for later identification.
fn handle_new_node(address: &SocketAddr, hostname: &String, hub_state: &HubState) -> u32 {
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
/// 
/// Updates relevant state in place.
fn handle_response(
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

/// Main thread function for remote node connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, hub_state: HubState) -> Result<()> {
    let mut serial;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        serial = handle_new_node(&address, &hostname, &hub_state);
        println!("{address} ({hostname}) connected and was assigned serial {serial}");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    loop {
        let response = stream.read::<Response>()?;
        println!("{address} ({hostname}) responded {response}");

        let result = handle_response(serial, response, &hub_state);
        if let Err(error) = result {
            match error {
                ErrHubState::SerialNotRecognized => {
                    eprintln!("{address} ({hostname}) is not a recognized node");
                    serial = handle_new_node(&address, &hostname, &hub_state);
                }
            }
        }
    }
}

/// Main function for handling incoming remote node connnections.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: TcpListener, hub_state: HubState) -> () {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state: HubState = Arc::clone(&hub_state);

                thread::spawn(move || {
                    if let Err(e) = thread_main(stream, hub_state) {
                        eprintln!("{e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("{e}");
            }
        };
    }
}
