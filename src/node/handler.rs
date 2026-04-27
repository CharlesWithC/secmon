use anyhow::Result;
use crossbeam_channel::Sender;
use std::io::{BufRead, BufReader};
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
                                [line_label, line_command_parts @ ..] if label == line_label => {
                                    let line_command = line_command_parts.join("=");
                                    let line_command_parts =
                                        line_command.split_whitespace().collect::<Vec<_>>();
                                    let mut command = process::Command::new(line_command_parts[0]);
                                    command.args(line_command_parts[1..].to_vec());
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

/// Runs `journalctl -f` on `sshd` logs and sends updates through `sw_s`.
///
/// The child process is moved into `child_opt_mutex` once spawned.
///
/// This is a blocking function and does not exit until child process is killed.
pub fn run_journalctl_sshd(
    sw_s: &Sender<Response>,
    child_opt_mutex: &Arc<Mutex<Option<process::Child>>>,
) -> Result<()> {
    let mut child = process::Command::new("journalctl")
        .args([
            "-t",
            "-sshd", // old sshd identifier
            "-t",
            "sshd-session", // modern sshd identifier
            "-f",           // block and print updates
            "-n",
            "0",  // ignore past log
            "-o", // set timestamp format
            "short-iso",
            "-q", // stay quiet on warning message
        ])
        .stdout(process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();

    let mut guard = child_opt_mutex.lock().unwrap();
    *guard = Some(child);
    drop(guard);

    let reader = BufReader::new(stdout);
    for line in reader.lines().map_while(Result::ok) {
        if let Some(log) = crate::node::data::parse_sshd_log(line)? {
            sw_s.send(Response::NodeUpdate(NodeUpdate {
                sessions: None,
                wg_peers: None,
                auth_log: Some(log),
            }))?;
        }
    }

    wait_child!(child_opt_mutex);

    Ok(())
}
