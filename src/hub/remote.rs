use anyhow::{Result, anyhow};
use crossbeam_channel::{Receiver, unbounded};
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::hub::{ChannelPacket, HubStateMutex};
use crate::models::node::Node;
use crate::models::nodestate::{NodeStateDiff, NodeStateError};
use crate::models::packet::Response;
use crate::models::{ASSUME_HOSTNAME_UNIQUE, DISCONNECT_GRACE_PERIOD};
use crate::traits::iosered::IOSerialized;

/// Initializes node connection.
///
/// Returns `serial` of the node and `receiver` for local commands.
fn handle_new_node(
    address: &SocketAddr,
    hostname: &String,
    hub_state: &HubStateMutex,
) -> (u32, Receiver<ChannelPacket>) {
    let mut guard = hub_state.lock().unwrap();
    let (ref mut counter, ref mut nodes) = *guard;

    // optionally remove possibly disconnected node of same hostname
    if ASSUME_HOSTNAME_UNIQUE {
        if let Some(index) = nodes
            .iter()
            .position(|(node, _)| node.hostname == *hostname)
        {
            nodes.remove(index);
        }
    }

    // increment counter for nodes
    // note: we must increment counter first to avoid serial 0
    // serial 0 is reserved for special use cases, such as referring to all connected nodes
    *counter += 1;

    // initialize node
    let node = Node {
        serial: *counter,
        address: address.clone(),
        hostname: hostname.clone(),
        sessions: Err(NodeStateError::Initializing),
        wg_peers: Err(NodeStateError::Initializing),
        last_state_update: UNIX_EPOCH,
        connected: true,
    };

    // create channels for client <=> node communication
    let (s, r) = unbounded::<ChannelPacket>();

    // add node and sender for commands to vector
    nodes.push((node, s));

    // return node serial and receiver for commands
    (*counter, r)
}

/// Updates hub state with node state difference.
///
/// Note: Full node state is not used to update hub state.
///
/// Returns new serial and command receiver if node cannot be recognized.
fn update_hub_state(
    serial: u32,
    address: &SocketAddr,
    hostname: &String,
    diff: NodeStateDiff,
    hub_state: &HubStateMutex,
) -> Option<(u32, Receiver<ChannelPacket>)> {
    let mut guard = hub_state.lock().unwrap();
    let (_, ref mut nodes) = *guard;
    if let Some(index) = nodes.iter().position(|(node, _)| node.serial == serial) {
        macro_rules! update_node_diff {
            ( $node:expr, $diff:expr, [$( $attr:ident ),*] ) => {
                $(if let Some($attr) = diff.$attr {
                    $node.$attr = $attr;
                })*
            }
        }

        let node = &mut nodes[index].0;
        update_node_diff!(node, diff, [sessions, wg_peers]);
        node.last_state_update = SystemTime::now();

        None
    } else {
        eprintln!("{address} ({hostname}) is not a recognized node");
        Some(handle_new_node(address, hostname, hub_state))
    }
}

/// Handles closed stream gracefully by dropping local command sender.
///
/// When local command sender is dropped, the thread handling local commands
/// would error and terminate.
fn handle_stream_close(serial: u32, hub_state: &HubStateMutex) -> () {
    let mut guard = hub_state.lock().unwrap();
    let (_, ref mut nodes) = *guard;
    if let Some(index) = nodes.iter().position(|(node, _)| node.serial == serial) {
        // create a dummy new channel, and replace original sender to drop it
        let (s, _) = unbounded::<ChannelPacket>();
        nodes[index].1 = s;
    }
    // note: it's fine if we cannot find the node - that tells the old receiver is already dropped
}

/// Main thread function for remote node connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, hub_state: &HubStateMutex) -> Result<()> {
    let serial;
    let cmd_receiver;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        (serial, cmd_receiver) = handle_new_node(&address, &hostname, hub_state);
        println!("{address} ({hostname}) connected and was assigned serial {serial}");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    thread::scope(|s| {
        // sender & receiver for stream reader (`sr`)
        // everything received by stream would be either handled locally or sent to `sr_s`
        // command handler should use `sr_r` to receive command responses
        let (sr_s, sr_r) = unbounded::<Response>();

        {
            // thread that handles local commands
            // note: `sr_r` is moved here; this is the only stream "writing" thread
            // note: `terminate` is not needed (and cannot be used, since we have a blocking `recv`)
            //       we drop the `cmd_sender` when connection closes, which makes `cmd_receiver` error,
            //       and then the thread would terminate for free
            let mut sw: TcpStream = stream
                .try_clone()
                .map_err(|e| anyhow!("Unable to clone stream: {e}"))?;

            s.spawn(move || -> Result<()> {
                loop {
                    // note: it is safe to receive a response with a blocking `recv`
                    // because we send the commands in serial order, and the node would
                    // only respond in matching serial order
                    // that said, we won't receive a response that doesn't match command
                    let (command, resp_sender) = cmd_receiver.recv()?;
                    sw.write(&command)?;
                    let response = sr_r.recv()?;
                    resp_sender.send(response)?;
                }
            });
        }

        // main function that handles stream read
        // note: `stream` and `sr_s` are moved here; this is the only stream "reading" function
        // note: this is a blocking function that terminates on stream close
        let result = move || -> Result<()> {
            loop {
                let response = stream.read::<Response>()?;
                println!("{address} ({hostname}) responded {response}");

                match response {
                    Response::KeepAlive => {}  // don't care
                    Response::Connect(_) => {} // should not occur
                    Response::NodeStateDiff(diff) => {
                        // use diff to update hub state
                        // diff is never requested by a command
                        update_hub_state(serial, &address, &hostname, diff, &hub_state);
                    }
                    response @ _ => {
                        // other response, including full node state
                        // send to command handler on a blocking `sr_r.recv()`
                        sr_s.send(response)?;
                    }
                }
            }
        }();

        // close the stream to drop `cmd_sender` to terminate the command handling thread
        handle_stream_close(serial, &hub_state);

        result
    })
}

/// Main function for handling incoming remote node connnections.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(listener: TcpListener, hub_state: HubStateMutex) -> () {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let hub_state: HubStateMutex = Arc::clone(&hub_state);

                thread::spawn(move || {
                    let address = stream.peer_addr().unwrap();

                    if let Err(e) = thread_main(stream, &hub_state) {
                        eprintln!("{e}");
                    }

                    // update `hub_state` to indicate disconnection
                    let mut guard = hub_state.lock().unwrap();
                    let (_, ref mut nodes) = *guard;
                    nodes.iter_mut().for_each(|(node, _)| {
                        if node.address == address {
                            node.connected = false;
                        }
                    });
                    drop(guard); // explicitly drop guard to unlock mutex

                    thread::sleep(Duration::from_secs(DISCONNECT_GRACE_PERIOD));

                    // delete node from `hub_state` after grace period
                    let mut guard = hub_state.lock().unwrap();
                    let (_, ref mut nodes) = *guard;
                    if let Some(index) = nodes.iter().position(|(node, _)| node.address == address)
                    {
                        nodes.remove(index);
                    } // note: `ASSUME_HOSTNAME_UNIQUE` can lead to early removal
                });
            }
            Err(e) => {
                eprintln!("{e}");
            }
        };
    }
}
