use anyhow::Result;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::iosered::IOSerialized;
use crate::models::hub::{ErrHubState, HubState};
use crate::models::node::Node;
use crate::models::packet::{Command, Response};

mod handler;

/// Main thread function for each node connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, hub_state: HubState) -> Result<()> {
    let mut serial;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        serial = handler::handle_new_node(&address, &hostname, &hub_state);
        println!("{address} ({hostname}) connected and was assigned serial {serial}");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    let command = Command::StateSyncStart;
    println!("Sending {command} to {address} ({hostname})");
    stream.write(&command)?;

    loop {
        let response = stream.read::<Response>()?;
        println!("{address} ({hostname}) responded {response}");

        let result = handler::handle_response(serial, response, &hub_state);
        if let Err(error) = result {
            match error {
                ErrHubState::SerialNotRecognized => {
                    eprintln!("{address} ({hostname}) is not a recognized node");
                    serial = handler::handle_new_node(&address, &hostname, &hub_state);
                }
            }
        }
    }
}

/// Hub main function to communicate with node.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(ip: IpAddr, port: u16) -> Result<()> {
    let listener = TcpListener::bind((ip, port))?;
    println!("Hub listening on {ip}:{port}");

    // mutex = (counter: u32, nodes: Vec(Node))
    // 'counter' helps find the entry in the vector for the node
    let hub_state: HubState = Arc::new(Mutex::new((0, Vec::<Node>::new())));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state = Arc::clone(&hub_state);

                thread::spawn(move || {
                    if let Err(e) = thread_main(stream, hub_state) {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("TcpStream error: {}", e);
            }
        };
    }

    Ok(())
}
