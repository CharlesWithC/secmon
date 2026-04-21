use anyhow::Result;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;

use crate::iosered::IOSerialized;
use crate::models::hub::{CtrlCmd, CtrlRes, HubStateMutex};

/// Handles local cli command.
///
/// Returns the result of executing the command.
fn handle_command(command: CtrlCmd, hub_state: &HubStateMutex) -> CtrlRes {
    match command {
        CtrlCmd::List => {
            let guard = hub_state.lock().unwrap();
            let (_, ref nodes) = *guard;
            CtrlRes::List(nodes.clone())
        }
    }
}

/// Main thread function for local cli connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: UnixStream, hub_state: HubStateMutex) -> Result<()> {
    loop {
        let command = stream.read::<CtrlCmd>()?;
        println!("Received {command}");

        let result = handle_command(command, &hub_state);
        stream.write(&result)?;

        // note: currently, cli only sends one single command in one connection
        // in the future, cli may become interactive where multiple commands may
        // be sent in one connection.
        // we return Ok(()) directly after first interaction for now, to avoid
        // connection closed error.
        return Ok(());
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
