use anyhow::Result;
use gethostname::gethostname;
use std::net::{IpAddr, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

mod handler;
mod state;
use crate::iosered::IOSerialized;
use crate::models::NodeConfig;
use crate::models::nodestate::NodeState;
use crate::models::packet::{Command, Response};

/// The real main function that handles commands and responses.
///
/// This is a blocking function and does not exit unless interrupted.
fn worker(ip: IpAddr, port: u16, node_state_receiver: Receiver<NodeState>) -> Result<()> {
    let mut stream = TcpStream::connect((ip, port))?;
    println!("Connected to hub {ip}:{port}");

    // use nonblocking to reduce complexity and send keep-alive messages
    stream.set_nonblocking(true)?;

    // respond hostname on new connection
    stream.write(&Response::Connect(
        gethostname()
            .to_str()
            .map(|v| v.to_owned())
            .unwrap_or(String::new()),
    ))?;

    // some local states
    let mut node_state: NodeState = (
        Err("Initializing".to_owned()),
        Err("Initializing".to_owned()),
    );
    let mut last_keepalive = SystemTime::now();

    loop {
        let mut command_opt = None;

        let mut _buf = [0u8; 4];
        match stream.peek(&mut _buf) {
            Ok(_) => command_opt = Some(stream.read::<Command>()?),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => Err(e)?,
        };

        if let Some(command) = command_opt {
            println!("Received {command}");
            handler::handle_command(&mut stream, &command, &node_state)?;
        }

        if let Ok(new_node_state) = node_state_receiver.recv_timeout(Duration::from_millis(100)) {
            node_state = new_node_state;
            let (sessions, wg_peers) = &node_state;
            stream.write(&Response::NodeState(sessions.clone(), wg_peers.clone()))?;
        }

        if SystemTime::now() - Duration::from_secs(60) >= last_keepalive {
            stream.write(&Response::KeepAlive)?;
            last_keepalive = SystemTime::now();
        }
    }
}

/// Node-side main function to communicate with hub.
///
/// This function initializes `node_state` and threads, then hands off to `worker`.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(ip: IpAddr, port: u16, node_config: NodeConfig) -> Result<()> {
    loop {
        let result = thread::scope(|s| {
            let terminate_flag = Arc::new(Mutex::new(false));
            let (node_state_sender, node_state_receiver) = channel::<NodeState>();

            {
                // thread that keeps track of node state changes
                // when node state updates, send a message through the channel
                // if worker is finished, then kill this thread to save resources
                let terminate_flag = Arc::clone(&terminate_flag);
                s.spawn(move || -> Result<()> {
                    // local node state tracker, not directly shared with worker
                    let mut node_state: NodeState = (
                        Err("Initializing".to_owned()),
                        Err("Initializing".to_owned()),
                    );
                    loop {
                        if *terminate_flag.lock().unwrap() {
                            // need to unwrap inside for (obvious) scoping reasons
                            return Ok(());
                        }
                        let updated = handler::update_node_state(node_config, &mut node_state);
                        if updated {
                            node_state_sender.send(node_state.clone())?;
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                });
            }

            // we need to put stream-stuff in a worker
            // so that when stream closes, the error can be propagated here
            // which would then update terminate flag for thread clean up
            let result = worker(ip, port, node_state_receiver);
            *terminate_flag.lock().unwrap() = true;

            result // directly relay result
        });

        if let Err(e) = result {
            if !node_config.reconnect {
                // if no reconnect, then propagate error
                return Err(e);
            } else {
                // otherwise, print error here and reconnect
                eprintln!("{e}");
            }
        } else {
            // if no reconnect, then complete and return
            return Ok(());
        } // otherwise, reconnect

        println!("Reconnecting in 5 seconds...");
        thread::sleep(Duration::from_secs(5));
    }
}
