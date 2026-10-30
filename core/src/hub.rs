use anyhow::{Result, anyhow};
use std::net::TcpListener;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::{CONFIG, HubConfig, get_config};
use secmon::models::hub::{HubNodes, HubStateMutex, SubscribedClients};

mod client;
mod local;
mod remote;

// Flow of data between client <=> node
//  - client creates initiates `UnixStream` to `hub/local` and sends commands
//  - `hub/local` sends `Packet` to corresponding `hub/node`, using channel in `HubState`
//  - `hub/node` sends `Command` to remote `node`
//  - remote `node` returns `Response` to `hub/node`
//  - `hub/node` sends `Response` to `hub/local`, using `Sender<Packet>` provided by `hub/local` earlier

/// Hub daemon main function for handling remote node connections and local client commands.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main_daemon(socket_path: String) -> Result<()> {
    let listener_local = UnixListener::bind(&socket_path)
        .map_err(|e| anyhow!("Unable to bind {socket_path}: {e}"))?;
    println!("Listening on {socket_path} for client commands");

    let HubConfig {
        bind_ip: ip,
        bind_port: port,
        ..
    } = get_config!(hub);

    let listener_remote =
        TcpListener::bind((*ip, *port)).map_err(|e| anyhow!("Unable to bind {ip}:{port}: {e}"))?;
    println!("Listening on {ip}:{port} for nodes");

    let hub_state: HubStateMutex =
        Arc::new(Mutex::new((0, HubNodes::new(), SubscribedClients::new())));

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

/// Hub client main function for handling local client command.
///
/// The command is read from command line arguments.
///
/// This may be a blocking function depending on the command.
pub fn main_client(command: String, socket_path: String) -> Result<()> {
    match UnixStream::connect(socket_path) {
        Ok(ref mut stream) => client::main(command, stream),
        Err(e) => Err(anyhow!("Unable to connect to hub daemon: {e}")),
    }
}
