use anyhow::Result;
use crossbeam_channel::unbounded;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use crate::models::hub::{ClientCommand, ClientResponse, HubStateMutex};
use crate::models::node::NodeUpdate;
use crate::models::packet::{Command, Response};
use crate::traits::iosered::IOSerialized;

/// Returns the result of finding a node based on a string-based query.
fn find_node(query: String, hub_state: &HubStateMutex) -> ClientResponse {
    let guard = hub_state.lock().unwrap();
    let (_, ref nodes, _) = *guard;
    nodes
        .iter()
        .find(|(node, _)| {
            // query based on serial, hostname, address
            node.serial.to_string() == query
                || node.hostname == query
                || node.address.to_string().split(":").next() == Some(&query)
        })
        .map(|(node, _)| ClientResponse::Node(node.clone()))
        .unwrap_or(ClientResponse::Failure(format!(
            "unable to identify node with '{query}'"
        )))
}

/// Handles forwarding a raw command to a given node, and relaying the node's response.
///
/// Note: Hub doesn't set expire time, or execute timeout for a command. These should be
/// handled by client and node. Hub would only act as a data transporter.
fn handle_raw_command(
    mut stream: &UnixStream,
    (serial, command, expire_time): (u32, Command, SystemTime),
    hub_state: &HubStateMutex,
) -> Result<()> {
    let guard = hub_state.lock().unwrap();
    let (_, ref nodes, _) = *guard;

    if let Some((_, sender)) = nodes.iter().find(|(node, _)| node.serial == serial) {
        let (resp_s, resp_r) = unbounded::<Response>();
        if let Err(e) = sender.send((command, resp_s, expire_time)) {
            stream.write(&ClientResponse::Failure(format!("{e}")))?;
            return Ok(());
        }
        drop(guard);

        loop {
            let resp = resp_r.recv()?;
            let is_streaming = crate::utils::is_streaming_response(&resp);
            stream.write(&ClientResponse::RawResponse(resp))?;
            if !is_streaming {
                break;
            }
        }
    } else {
        stream.write(&ClientResponse::Failure(format!(
            "SERIAL={serial} is not a recognized node"
        )))?;
    }

    Ok(())
}

/// Main thread function for a single local client connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: UnixStream, hub_state: HubStateMutex) -> Result<()> {
    loop {
        let command = stream.read::<ClientCommand>()?;
        println!("Received {command}");

        match command {
            ClientCommand::Subscribe => {
                let (s, r) = unbounded::<(u32, NodeUpdate)>();

                let mut guard = hub_state.lock().unwrap();
                let (_, _, ref mut subscribers) = *guard;
                subscribers.push(s);
                drop(guard);

                loop {
                    let (serial, data) = r.recv()?;
                    stream.write(&ClientResponse::NodeUpdate(serial, data))?;
                }

                // no need to try to remove subscriber
                // remote would remove zombie subscribers automatically
            }
            ClientCommand::List => {
                let guard = hub_state.lock().unwrap();
                let (_, ref nodes, _) = *guard;
                stream.write(&ClientResponse::List(
                    nodes.into_iter().map(|(node, _)| node.clone()).collect(),
                ))?;
            }
            ClientCommand::FindNode(query) => {
                let resp = find_node(query, &hub_state);
                stream.write(&resp)?;
            }
            ClientCommand::RawCommand(serial, command, expire_time) => {
                handle_raw_command(&stream, (serial, command, expire_time), &hub_state)?;
            }
        }
    }
}

/// Main function listening for incoming local client connections.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: UnixListener, hub_state: HubStateMutex) -> () {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state: HubStateMutex = Arc::clone(&hub_state);

                thread::spawn(move || {
                    if let Err(e) = thread_main(stream, hub_state) {
                        eprintln!("Error while handling local connection: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting local connection: {e}");
            }
        };
    }
}
