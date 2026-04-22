use anyhow::Result;
use crossbeam_channel::unbounded;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;

use crate::iosered::IOSerialized;
use crate::models::hub::{CtrlCmd, CtrlRes, HubStateMutex};
use crate::models::packet::Response;

/// Handles local cli command.
///
/// Returns the result of executing the command.
fn handle_command(command: CtrlCmd, hub_state: &HubStateMutex) -> CtrlRes {
    match command {
        CtrlCmd::List => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes) = *guard;
            CtrlRes::List(nodes.into_iter().map(|(node, _)| node.clone()).collect())
        }
        CtrlCmd::FindNode(query) => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes) = *guard;

            nodes
                .iter()
                .find(|(node, _)| {
                    // query based on hostname, then address
                    node.hostname == query
                        || node.address.to_string().split(":").next() == Some(&query)
                })
                .map(|(node, _)| CtrlRes::Node(node.clone()))
                .unwrap_or(CtrlRes::Failure(format!(
                    "unable to identify node with '{query}'"
                )))
        }
        CtrlCmd::RawCommand(serial, command) => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes) = *guard;

            if let Some((_, sender)) = nodes.iter().find(|(node, _)| node.serial == serial) {
                let (resp_s, resp_r) = unbounded::<Response>();
                if let Err(e) = sender.send((command, resp_s)) {
                    return CtrlRes::Failure(format!("{e}"));
                }
                let resp_res = resp_r.recv();
                match resp_res {
                    Ok(resp) => CtrlRes::RawResponse(resp),
                    Err(e) => CtrlRes::Failure(format!("{e}")),
                }
            } else {
                CtrlRes::Failure(format!("SERIAL={serial} is not a recognized node"))
            }
        }
        CtrlCmd::Quit => panic!("`CtrlCmd::Quit` should not be handled by `handle_command`"),
    }
}

/// Main thread function for local cli connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: UnixStream, hub_state: HubStateMutex) -> Result<()> {
    loop {
        let command = stream.read::<CtrlCmd>()?;
        println!("Received {command}");
        if let CtrlCmd::Quit = command {
            return Ok(());
        }

        let result = handle_command(command, &hub_state);
        stream.write(&result)?;
    }
}

/// Main function for handling incoming local cli connections.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: UnixListener, hub_state: HubStateMutex) -> () {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state: HubStateMutex = Arc::clone(&hub_state);

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
