use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::hub::{ChannelPacket, HubStateMutex};
use crate::models::node::Node;
use crate::models::nodestate::NodeStateError;
use crate::models::packet::Response;
use crate::models::{ASSUME_HOSTNAME_UNIQUE, DISCONNECT_GRACE_PERIOD};
use crate::traits::iosered::IOSerialized;

/// Initializes node connection.
///
/// Returns `serial` of the node and `receiver` for internal channel packets.
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

/// Main thread function for remote node connection.
///
/// This is a blocking function and does not exit unless interrupted.
fn thread_main(mut stream: TcpStream, hub_state: &HubStateMutex) -> Result<()> {
    let mut serial;
    let mut receiver;
    let address = stream.peer_addr().unwrap();
    let hostname;

    if let Response::Connect(_hostname) = stream.read::<Response>()? {
        hostname = _hostname;
        (serial, receiver) = handle_new_node(&address, &hostname, hub_state);
        println!("{address} ({hostname}) connected and was assigned serial {serial}");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid initial response: Should be Response::Connect"
        ));
    }

    // use read-timeout stream, and non-blocking channel
    // reason: we prioritize node state update over local commands
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;

    // send the next non-`KeepAlive` non-`NodeState` `Response` here
    let mut respond_command_response_to: Option<Sender<Response>> = None;

    loop {
        // only handle new local command when last command is handled
        // note that, commands are sent serially, and responds are received serially,
        // and so there is no risk of race condition / mismatch of command and response
        if let None = respond_command_response_to {
            // peek for a local command, do not block
            let mut chanpkt_opt: Option<ChannelPacket> = None;
            match receiver.try_recv() {
                Ok(chanpkt) => chanpkt_opt = Some(chanpkt),
                Err(TryRecvError::Empty) => {}
                Err(e) => Err(e)?,
            }

            if let Some((command, sender)) = chanpkt_opt {
                stream.write(&command)?;
                respond_command_response_to = Some(sender);
            }
        }

        // try to receive a response from node with read timeout
        // if it's a NodeState, then we happily accept it and update `hub_state`
        // if it's some other response, then we send it back to the original command sender through channel
        let mut response_opt = None;
        let mut _buf = [0u8; 4];
        match stream.read::<Response>() {
            // note: stream.read blocks until timeout
            Ok(response) => response_opt = Some(response),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => Err(e)?,
        };

        if let Some(response) = response_opt {
            // there is some response
            println!("{address} ({hostname}) responded {response}");

            match response {
                Response::KeepAlive => {}  // don't care
                Response::Connect(_) => {} // should not occur
                Response::NodeState(node_state) => {
                    let mut guard = hub_state.lock().unwrap();
                    let (_, ref mut nodes) = *guard;
                    if let Some(index) = nodes.iter().position(|(node, _)| node.serial == serial) {
                        macro_rules! update_node {
                            ( $node:expr, $updated:expr, [$( $attr:ident ),*] ) => {
                                $($node.$attr = $updated.$attr;)*
                            }
                        }

                        let node = &mut nodes[index].0;
                        update_node!(node, node_state, [sessions, wg_peers]);
                        node.last_state_update = SystemTime::now();
                    } else {
                        // for whatever reason, the node got removed from `hub_state`
                        // note: this should NOT occur but we handle it gracefully here by reinitializing the node
                        eprintln!("{address} ({hostname}) is not a recognized node");
                        (serial, receiver) = handle_new_node(&address, &hostname, hub_state);
                    }
                }
                Response::NodeStateDiff(diff) => {
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
                    } else {
                        eprintln!("{address} ({hostname}) is not a recognized node");
                        (serial, receiver) = handle_new_node(&address, &hostname, hub_state);
                    }
                }
                response @ _ => {
                    // other response, send to original command sender through channel
                    if let Some(sender) = respond_command_response_to {
                        sender.send(response)?;
                        respond_command_response_to = None; // clear sender to accept next command
                    }
                }
            }
        }
    }
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
