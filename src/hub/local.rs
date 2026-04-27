use anyhow::Result;
use crossbeam_channel::unbounded;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;

use crate::models::hub::{ClientCommand, ClientResponse, HubStateMutex};
use crate::models::node::NodeStateDiff;
use crate::models::packet::Response;
use crate::traits::iosered::IOSerialized;

/// Handles local client command.
///
/// Returns the result of executing the command.
fn handle_command(command: ClientCommand, hub_state: &HubStateMutex) -> ClientResponse {
    match command {
        ClientCommand::List => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes, _) = *guard;
            ClientResponse::List(nodes.into_iter().map(|(node, _)| node.clone()).collect())
        }
        ClientCommand::FindNode(query) => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes, _) = *guard;

            nodes
                .iter()
                .find(|(node, _)| {
                    // query based on hostname, then address
                    node.hostname == query
                        || node.address.to_string().split(":").next() == Some(&query)
                })
                .map(|(node, _)| ClientResponse::Node(node.clone()))
                .unwrap_or(ClientResponse::Failure(format!(
                    "unable to identify node with '{query}'"
                )))
        }
        ClientCommand::RawCommand(serial, command) => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes, _) = *guard;

            if let Some((_, sender)) = nodes.iter().find(|(node, _)| node.serial == serial) {
                let (resp_s, resp_r) = unbounded::<Response>();
                if let Err(e) = sender.send((command, resp_s)) {
                    return ClientResponse::Failure(format!("{e}"));
                }
                let resp_res = resp_r.recv();
                match resp_res {
                    Ok(resp) => ClientResponse::RawResponse(resp),
                    Err(e) => ClientResponse::Failure(format!("{e}")),
                }
            } else {
                ClientResponse::Failure(format!("SERIAL={serial} is not a recognized node"))
            }
        }
        ClientCommand::Subscribe | ClientCommand::Quit => {
            panic!("`{command}` should not be handled by `handle_command`")
        }
    }
}

/// Handles client subscription creation and data forwarding.
fn handle_subscribe(mut stream: UnixStream, hub_state: &HubStateMutex) -> Result<()> {
    let (s, r) = unbounded::<(u32, NodeStateDiff)>();

    let mut guard = hub_state.lock().unwrap();
    let (_, _, ref mut subscribers) = *guard;
    subscribers.push(s);
    drop(guard);

    loop {
        let (serial, diff) = r.recv()?;
        stream.write(&ClientResponse::NodeStateDiff(serial, diff))?;
    }

    // no need to try to remove subscriber
    // remote would remove zombie subscribers automatically
}

/// Main thread function for local client connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: UnixStream, hub_state: HubStateMutex) -> Result<()> {
    loop {
        let command = stream.read::<ClientCommand>()?;
        println!("Received {command}");

        match command {
            ClientCommand::Quit => {
                return Ok(());
            }
            ClientCommand::Subscribe => {
                handle_subscribe(stream, &hub_state)?;
                return Ok(());
            }
            command @ _ => {
                let result = handle_command(command, &hub_state);
                stream.write(&result)?
            }
        }
    }
}

/// Main function for handling incoming local client connections.
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
