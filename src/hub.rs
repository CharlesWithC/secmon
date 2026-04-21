use anyhow::{Result, anyhow};
use std::net::{IpAddr, TcpListener};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::models::hub::HubStateMutex;
use crate::models::node::Node;

mod local;
mod remote;

/// Hub main function for handling remote node connections and local control commands.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(ip: IpAddr, port: u16, socket_path: String) -> Result<()> {
    let listener_local = UnixListener::bind(&socket_path)
        .map_err(|e| anyhow!("Unable to bind {socket_path}: {e}"))?;
    println!("Listening on {socket_path} for control commands");

    let listener_remote =
        TcpListener::bind((ip, port)).map_err(|e| anyhow!("Unable to bind {ip}:{port}: {e}"))?;
    println!("Listening on {ip}:{port} for nodes");

    // mutex = (counter: u32, nodes: Vec(Node))
    // 'counter' helps find the entry in the vector for the node
    let hub_state: HubStateMutex = Arc::new(Mutex::new((0, Vec::<Node>::new())));

    thread::scope(|s| {
        let hub_state_local: HubStateMutex = Arc::clone(&hub_state);
        s.spawn(move || {
            local::main(listener_local, hub_state_local);
        });

        let hub_state_remote: HubStateMutex = Arc::clone(&hub_state);
        s.spawn(move || {
            remote::main(listener_remote, hub_state_remote);
        });
    });

    Ok(())
}
