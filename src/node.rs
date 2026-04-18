use anyhow::Result;
use gethostname::gethostname;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod handler;
mod state;
use crate::iosered::IOSerialized;
use crate::models::Mode;
use crate::models::node::NodeState;
use crate::models::packet::{Command, Response};
use crate::node::handler::{handle_command, update_node_state};

/// Node-side main function to communicate with hub.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(mut stream: TcpStream, mode: Mode) -> Result<()> {
    // use nonblocking to reduce complexity and send keep-alive messages
    stream.set_nonblocking(true)?;

    let node_state: NodeState = Arc::new(Mutex::new((
        Err("Initializing".to_owned()),
        Err("Initializing".to_owned()),
        UNIX_EPOCH,
    )));
    {
        // start thread to update node state in the background
        let node_state = Arc::clone(&node_state);
        thread::spawn(move || {
            loop {
                update_node_state(&node_state, &mode);
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    // respond hostname on new connection
    stream.write(&Response::Connect(
        gethostname()
            .to_str()
            .map(|v| v.to_owned())
            .unwrap_or(String::new()),
    ))?;

    // initialize variables for state_sync feature
    let mut state_sync = false;
    let mut state_sync_last_update = UNIX_EPOCH;
    let mut last_keepalive = SystemTime::now();

    loop {
        let mut command_opt = None;

        let mut _buf = [0u8; 4];
        match stream.peek(&mut _buf) {
            Ok(_) => command_opt = Some(stream.read::<Command>()?),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("encountered IO error: {e}"),
        };

        if let Some(command) = command_opt {
            println!("Received {command}");
            handle_command(&mut stream, &command, &node_state, &mut state_sync)?;
        }

        // hub should deal with state_sync response gracefully,
        // in case a node state is responded while another command is being sent
        // i.e. hub should quietly update node state even if it's expecting different response
        if state_sync {
            let guard = node_state.lock().unwrap();
            let (ref sessions, ref wg_peers, ref update_time) = *guard;
            // only sync if there is an update
            if *update_time > state_sync_last_update {
                stream.write(&Response::NodeState(sessions.clone(), wg_peers.clone()))?;
                state_sync_last_update = *update_time;
                last_keepalive = SystemTime::now();
            }
        }

        if SystemTime::now() - Duration::from_secs(60) >= last_keepalive {
            stream.write(&Response::KeepAlive)?;
            last_keepalive = SystemTime::now();
        }

        thread::sleep(Duration::from_secs(1));
    }
}
