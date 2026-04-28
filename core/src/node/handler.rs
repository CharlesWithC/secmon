use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use std::io::{BufRead, BufReader, Read};
use std::process;
use std::sync::{Arc, Mutex};

use crate::models::NodeConfig;
use crate::models::node::{NodeDataError, NodeStateMutex, NodeUpdate};
use crate::models::packet::{Command, Response};
use crate::traits::exec::Exec;
use crate::utils::{get_env_var, read_lines};

/// Handles command from hub.
pub fn handle_command(
    sw_s: &Sender<Response>,
    command: &Command,
    node_state_mutex: &NodeStateMutex,
) -> Result<()> {
    match command {
        Command::NodeState => {
            let node_state = node_state_mutex.lock().unwrap();
            sw_s.send(Response::NodeState(node_state.clone()))?;
        }
        Command::Execute(label) => {
            match get_env_var::<String>("COMMAND_ALLOWLIST_FILE", None).unwrap() {
                None => {
                    sw_s.send(Response::Result(
                        false,
                        "Command allowlist not set (Missing env var: COMMAND_ALLOWLIST_FILE)"
                            .to_owned(),
                    ))?;
                }
                Some(allowlist_file) => match read_lines(allowlist_file.clone()) {
                    Err(e) => {
                        sw_s.send(Response::Result(
                            false,
                            format!(
                                "Unable to read '{allowlist_file}' (COMMAND_ALLOWLIST_FILE): {:?}",
                                e
                            ),
                        ))?;
                    }
                    Ok(lines) => {
                        for line in lines.map_while(Result::ok) {
                            match line.split("=").collect::<Vec<_>>().as_slice() {
                                #[rustfmt::skip]
                                [line_label, line_command_parts @ ..] if label == line_label => {
                                    let line_command = line_command_parts.join("=");
                                    let parts_res = shlex::split(line_command.as_str());
                                    if let None = parts_res {
                                        sw_s.send(Response::Result(false, format!("Failed to parse command for '{line_label}'")))?;
                                        return Ok(());
                                    }
                                    let parts = parts_res.unwrap();
                                    let mut command = process::Command::new(parts[0].as_str());
                                    command.args(parts[1..].to_vec());
                                    let result = command.run();
                                    sw_s.send(match result {
                                        Ok(output) => Response::Result(true, output),
                                        Err(error) => Response::Result(false, error),
                                    })?;
                                    return Ok(()); // early return on match
                                }
                                _ => {}
                            }
                        }

                        // no match
                        sw_s.send(Response::Result(
                            false,
                            format!("'{label}' is not an allowed command"),
                        ))?;
                    }
                },
            }
        }
    };

    Ok(())
}

/// Fetches and updates stored `node_state` in place.
///
/// Returns whether some update is made, and atomic update of new state.
pub fn update_node_state(
    node_config: NodeConfig,
    node_state_mutex: &NodeStateMutex,
) -> (bool, NodeUpdate) {
    /// Macro to fetch updates based on `node_config`.
    macro_rules! fetch_updates {
        ( $( $attr:ident ),* ) => {
            paste::paste! {
            $(let $attr = if node_config.[<enable_ $attr>] {
                let result = crate::node::data::[<get_ $attr>]();
                match result {
                    // repack the result to use NodeStateError
                    Ok(result) => Ok(result),
                    Err(e) => Err(NodeDataError::Message(e)),
                }
            } else {
                Err(NodeDataError::NotMonitored)
            };)*
        }}
    }
    fetch_updates!(sessions, wg_peers);

    /// Macro to compare and update node state.
    ///
    /// Returns a tuple of whether some update is made, and atomic update of new state.
    macro_rules! cmp_upd_state {
        ( $node_state:expr, [$( $attr:ident ),*] ) => {{
            let mut data = NodeUpdate {
                $($attr: None,)*
                auth_log: None, // auth_log is not monitored here
            };
            let mut updated = false;
            $(if $attr != $node_state.$attr {
                $node_state.$attr = $attr.clone();
                data.$attr = Some($attr);
                updated = true;
            })*
            (updated, data)
        }}
    }

    let mut node_state = node_state_mutex.lock().unwrap();
    cmp_upd_state!(node_state, [sessions, wg_peers])
}

/// Macro to try to wait for a child to exit, and ignore error if fails.
macro_rules! wait_child {
    ( $child_opt_mutex:expr ) => {
        let ref mut child_opt = *$child_opt_mutex.lock().unwrap();
        match child_opt {
            Some(child) => {
                let _ = child.wait();
            }
            _ => {}
        }
        *child_opt = None;
    };
}

/// Runs `journalctl` as a child process and sends parsed updates through `sw_s`.
///
/// The child process is moved into `child_opt_mutex` once spawned.
///
/// This is a blocking function and does not exit until child process is killed.
fn run_journalctl(
    node_config: NodeConfig,
    sw_s: &Sender<Response>,
    child_opt_mutex: &Arc<Mutex<Option<process::Child>>>,
) -> Result<()> {
    let mut args = vec![
        "-f", // block and print updates
        "-n",
        "0",  // ignore past log
        "-o", // set timestamp format
        "short-iso",
        "-q", // stay quiet on warning message
    ]; // base args
    if node_config.enable_auth_log {
        args.extend([
            "-t",
            "sshd", // old sshd identifier
            "-t",
            "sshd-session", // modern sshd identifier
            "-t",
            "su", // su sessions
        ]);
    }

    let mut child = process::Command::new("journalctl")
        .args(args)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn journalctl child process: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or(anyhow!("Failed to capture journalctl stdout"))?;

    let stderr = child
        .stderr
        .take()
        .ok_or(anyhow!("Failed to capture journalctl stderr"))?;

    let mut guard = child_opt_mutex.lock().unwrap();
    *guard = Some(child);
    drop(guard);

    let reader = BufReader::new(stdout);
    for line in reader.lines().map_while(Result::ok) {
        if let Some(log) = crate::node::data::parse_journalctl_log(line)? {
            let result = sw_s.send(Response::NodeUpdate(NodeUpdate {
                sessions: None,
                wg_peers: None,
                auth_log: Some(Ok(log)),
            }));
            if let Err(_) = result {
                // channel closed, return
                // error should only be propagated on "actual" error (e.g. when collecting/parsing data)
                return Ok(());
            }
        }
    }

    wait_child!(child_opt_mutex);

    let mut reader = BufReader::new(stderr);
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| anyhow!("Unable to process journalctl stderr: {e}"))?;
    if buf.trim().len() != 0 {
        let _ = sw_s.send(Response::NodeUpdate(NodeUpdate {
            sessions: None,
            wg_peers: None,
            auth_log: Some(Err(NodeDataError::Message(format!(
                "Error executing journalctl: {buf}"
            )))),
        }));
    }

    Ok(())
}

/// Handles journalctl log collection and error relaying.
///
/// Returns `true` if no error is raised, otherwise `false`.
///
/// This is a blocking function and does not exit until child process is killed.
pub fn handle_journalctl(
    node_config: NodeConfig,
    sw_s: &Sender<Response>,
    child_opt_mutex: &Arc<Mutex<Option<process::Child>>>,
) -> bool {
    let result = run_journalctl(node_config, sw_s, child_opt_mutex);
    if let Err(e) = result {
        let _ = sw_s.send(Response::NodeUpdate(NodeUpdate {
            sessions: None,
            wg_peers: None,
            auth_log: Some(Err(NodeDataError::Message(format!(
                "Error processing journalctl data: {e}"
            )))),
        }));
        return false;
    }
    return true;
}
