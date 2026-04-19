use anyhow::{Result, anyhow};
use std::fs;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::iosered::IOSerialized;
use crate::models::DEFAULT_SOCKET_DIR;
use crate::models::hub::{ErrHubState, HubState};
use crate::models::node::Node;
use crate::models::packet::Response;
use crate::utils::get_env_var;

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
    let mut socket_path = DEFAULT_SOCKET_DIR.to_owned() + "/secmon.sock";
    if let Some(dir) = get_env_var::<String>("XDG_RUNTIME_DIR", None) {
        if !dir.ends_with("/0") {
            // non-root
            socket_path = dir + "/secmon.sock";
        }
    }

    if fs::exists(&socket_path).map_err(|e| anyhow!("Unable to access {socket_path}: {e}"))? {
        fs::remove_file(&socket_path)
            .map_err(|e| anyhow!("Unable to unlink {socket_path}: {e}"))?;
    }

    let listener =
        TcpListener::bind((ip, port)).map_err(|e| anyhow!("Unable to bind {ip}:{port}: {e}"))?;
    println!("Listening on {ip}:{port} for nodes");

    let _listener_ctrl = UnixListener::bind(&socket_path)
        .map_err(|e| anyhow!("Unable to bind {socket_path}: {e}"))?;
    println!("Listening on {socket_path} for control commands");

    // mutex = (counter: u32, nodes: Vec(Node))
    // 'counter' helps find the entry in the vector for the node
    let hub_state: HubState = Arc::new(Mutex::new((0, Vec::<Node>::new())));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state = Arc::clone(&hub_state);

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

    Ok(())
}
