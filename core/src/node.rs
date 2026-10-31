use anyhow::{Result, anyhow};
use crossbeam_channel::{bounded, unbounded};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

mod data;
mod handler;
use crate::config::{CONFIG, NodeConfig, get_config};
use crate::crypto::{
    NOISE_HANDSHAKE_MAXLEN, NOISE_HANDSHAKE_MAXTIME, NOISE_MAXLEN, NOISE_PATTERN, NoiseShared,
    NoisyStream,
};
use secmon::models::node::{KEEPALIVE_INTERVAL, NodeDataError, NodeState};
use secmon::models::packet::{Command, Response};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::deadline_watchdog;

/// Node-side main function to communicate with hub.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main() -> Result<()> {
    // only print error when last connect is successful
    // otherwise, retry silently, if applicable
    // note: initialize to true to print first error
    let mut last_connect_successful = true;
    let mut reconnect_wait = 100;
    loop {
        let node_config = get_config!(node);
        let NodeConfig {
            hub_ip: ip,
            hub_port: port,
            ..
        } = node_config;

        let result = thread::scope(|s| {
            let stream = TcpStream::connect((*ip, *port))?;

            // wrap stream
            // note: encryption is disabled during handshake as `noise=None`
            let mut stream = NoisyStream {
                stream,
                noise: None,
            };

            // noise protocol handeshake
            let noise_config = &get_config!(node).noise;
            if noise_config.enable {
                let mut noise = snow::Builder::new(NOISE_PATTERN.parse()?)
                    .local_private_key(&noise_config.private_key.as_ref())?
                    .remote_public_key(&noise_config.hub_public_key.as_ref())?
                    .psk(0, &noise_config.preshared_key.into())?
                    .build_initiator()?;
                let mut ct = [0u8; NOISE_MAXLEN]; // buffer for encrypted handshake data
                let mut pt = [0u8; NOISE_MAXLEN]; // empty buffer to make noise happy

                let killer = stream.stream.try_clone()?;
                let (watchdog_tx, watchdog_rx) = bounded::<()>(1);
                deadline_watchdog(
                    watchdog_rx,
                    NOISE_HANDSHAKE_MAXTIME,
                    || {},
                    move || {
                        println!(
                            "Hub failed to complete noise protocol handshake (timeout after {} seconds)",
                            NOISE_HANDSHAKE_MAXTIME.as_secs()
                        );
                        killer
                            .shutdown(Shutdown::Both)
                            .expect("shutdown should succeed");
                    },
                );

                // -> e, es
                let n = noise.write_message(&[], &mut ct)?;
                stream.write(&ct[..n].to_vec())?;
                // <- e, ee
                let buf = stream.read_upto::<Vec<u8>>(NOISE_HANDSHAKE_MAXLEN)?;
                noise.read_message(buf.as_slice(), &mut pt)?;
                // -> s, ee
                let n = noise.write_message(&[], &mut ct)?;
                stream.write(&ct[..n].to_vec())?;

                watchdog_tx.send(())?; // send watchdog signal once handshake is done

                if !noise.is_handshake_finished() {
                    return Err(anyhow::anyhow!(
                        "Failed to complete noise protocol handshake"
                    ));
                }

                stream.noise = Some(Arc::new(NoiseShared {
                    state: noise.into_stateless_transport_mode()?,
                    tx: Mutex::new(0),
                    rx: Mutex::new(0),
                }));
            }

            println!("Connected to hub {ip}:{port}");
            last_connect_successful = true;
            reconnect_wait = 100;

            // respond hostname on new connection
            stream.write(&Response::Connect {
                hostname: node_config.hostname.clone(),
            })?;

            // sender & receiver for stream writer (`sw`)
            // everything to be sent to stream should be sent to `sw_s`
            // anything received by `sw_r` is written to `stream`
            // this allows multiple threads to write to stream safely
            let (sw_s, sw_r) = unbounded::<Response>();

            // flag for terminating threads when main thread is done
            let terminate_flag = Arc::new(Mutex::new(false));

            // long-living child processes to be killed on termination
            let child_journalctl = Arc::new(Mutex::new(None));

            // current node state (we cache this to respond to `NodeState` command)
            let node_state_mutex = Arc::new(Mutex::new(NodeState {
                sessions: Err(NodeDataError::Initializing),
                wg_peers: Err(NodeDataError::Initializing),
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
                        let (updated, data) = handler::update_node_state(&node_state_mutex);
                        if updated {
                            sw_s.send(Response::NodeUpdate(data))?;
                        }

                        // handle keep alive
                        if SystemTime::now() - Duration::from_secs(KEEPALIVE_INTERVAL)
                            >= last_keepalive
                        {
                            sw_s.send(Response::KeepAlive)?;
                            last_keepalive = SystemTime::now();
                        }

                        thread::sleep(Duration::from_secs(1));
                    }

                    // `sw_s` is dropped when thread teminates on `terminate_flag`
                });
            }

            if node_config.resources.iter().any(|s| s == "auth") {
                // thread that tracks `journalctl` for updates
                // note: to terminate this thread, set `terminate_flag` to `true`
                //       and then kill the journalctl child process; killing without
                //       `terminate_flag=true` will cause respawning/retrying
                // note: currently only `auth` log relies on `journalctl`; if more types
                //       of logs are tracked in the future, update the condition of the
                //       `if` to trigger this code block on other `node_config`; the handler
                //       for `journalctl` would decide what logs to track based on `node_config`
                let sw_s = sw_s.clone();
                let terminate_flag = Arc::clone(&terminate_flag);
                let child_opt_mutex = Arc::clone(&child_journalctl);

                s.spawn(move || {
                    loop {
                        if *terminate_flag.lock().unwrap() {
                            return;
                        }

                        let success = handler::handle_journalctl(&sw_s, &child_opt_mutex);
                        if !success {
                            // error is relayed to hub; do not retry on error
                            return;
                        } // otherwise, child likely got terminated, continue with loop
                    }
                });
            }

            // main function that handles stream read and hub commands
            // note: `stream` and `sw_s` are moved here; this is the only stream "reading" function
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

            // kill child processes
            for mutex in [child_journalctl] {
                let ref mut child_opt = *mutex.lock().unwrap();
                match child_opt {
                    Some(child) => {
                        let _ = child.kill();
                    }
                    None => {}
                }
            }

            // directly relay result
            result
        });

        if let Err(e) = result {
            if !node_config.reconnect {
                // if no reconnect, then propagate error
                return Err(anyhow!("Error communicating with hub: {e}"));
            } else if last_connect_successful {
                // otherwise, print error here and reconnect
                eprintln!("Error communicating with hub: {e}");
            }
        } else {
            // if no reconnect, then complete and return
            return Ok(());
        } // otherwise, reconnect

        reconnect_wait += reconnect_wait; // exponential back-off
        reconnect_wait = std::cmp::min(reconnect_wait, 128000); // max wait
        println!("Reconnecting after {reconnect_wait}ms...");
        thread::sleep(Duration::from_millis(reconnect_wait));
        last_connect_successful = false;
    }
}
