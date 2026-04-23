use anyhow::{Result, anyhow};
use crossbeam_channel::unbounded;
use gethostname::gethostname;
use std::net::{IpAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

mod handler;
mod state;
use crate::models::NodeConfig;
use crate::models::nodestate::{NodeState, NodeStateError};
use crate::models::packet::{Command, Response};
use crate::traits::iosered::IOSerialized;

/// Node-side main function to communicate with hub.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(ip: IpAddr, port: u16, node_config: NodeConfig) -> Result<()> {
    // only print error when last connect is successful
    // otherwise, retry silently, if applicable
    // note: initialize to true to print first error
    let mut last_connect_successful = true;
    loop {
        let result = thread::scope(|s| {
            let mut stream = TcpStream::connect((ip, port))?;
            println!("Connected to hub {ip}:{port}");

            last_connect_successful = true;

            // respond hostname on new connection
            stream.write(&Response::Connect(
                gethostname()
                    .to_str()
                    .map(|v| v.to_owned())
                    .unwrap_or(String::new()),
            ))?;

            // sender & receiver for stream writer (`sw`)
            // everything to be sent to stream should be sent to `sw_s`
            // anything received by `sw_r` is written to `stream`
            // this allows multiple threads to write to stream safely
            let (sw_s, sw_r) = unbounded::<Response>();

            // flag for terminating threads when main thread is done
            let terminate_flag = Arc::new(Mutex::new(false));

            // current node state (we cache this to respond to `NodeState` command)
            let node_state_mutex = Arc::new(Mutex::new(NodeState {
                sessions: Err(NodeStateError::Initializing),
                wg_peers: Err(NodeStateError::Initializing),
            }));

            {
                // thread that handles stream write
                // note: `sw_r` is moved here; this is the only stream "writing" thread
                // note: `terminate_flag` is not needed here because `sw_r` will fail
                //       when all senders are closed (i.e. state update thread & worker)
                let mut sw = stream
                    .try_clone()
                    .map_err(|e| anyhow!("Unable to clone stream: {e}"))?;

                s.spawn(move || -> Result<()> {
                    loop {
                        let data = sw_r.recv()?;
                        sw.write(&data)?;
                    }
                });
            }

            {
                // thread that keeps track of node state changes and keep-alive
                // note: this thread relies on `terminate_flag` for termination
                // note: this thread does not deal with stream directly
                let sw_s = sw_s.clone();
                let terminate_flag = Arc::clone(&terminate_flag);
                let node_state_mutex = Arc::clone(&node_state_mutex);

                s.spawn(move || -> Result<()> {
                    let mut last_keepalive = SystemTime::now();
                    loop {
                        if *terminate_flag.lock().unwrap() {
                            return Ok(());
                        }

                        // update node state
                        let (updated, diff) =
                            handler::update_node_state(node_config, &node_state_mutex);
                        if updated {
                            sw_s.send(Response::NodeStateDiff(diff))?;
                        }

                        // handle keep alive
                        if SystemTime::now() - Duration::from_secs(60) >= last_keepalive {
                            sw_s.send(Response::KeepAlive)?;
                            last_keepalive = SystemTime::now();
                        }

                        thread::sleep(Duration::from_secs(1));
                    }

                    // `sw_s` is dropped when thread teminates on `terminate_flag`
                });
            }

            // main worker that handles stream read and hub commands
            // note: `sw_s` is moved here; this is the only stream "reading" function
            // note: this is a blocking function that terminates on stream close
            let result = move || -> Result<()> {
                loop {
                    let command = stream.read::<Command>()?;
                    println!("Received {command}");
                    handler::handle_command(&sw_s, &command, &node_state_mutex)?;
                }

                // `sw_s` is dropped when function returns
            }();

            // main worker is done, terminate other threads
            *terminate_flag.lock().unwrap() = true;

            // directly relay result
            result
        });

        if let Err(e) = result {
            if !node_config.reconnect {
                // if no reconnect, then propagate error
                return Err(e);
            } else if last_connect_successful {
                // otherwise, print error here and reconnect
                eprintln!("{e}");
            }
        } else {
            // if no reconnect, then complete and return
            return Ok(());
        } // otherwise, reconnect

        if last_connect_successful {
            println!("Reconnecting...");
        }
        thread::sleep(Duration::from_millis(100));
        last_connect_successful = false;
    }
}
