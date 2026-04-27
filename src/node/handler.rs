use anyhow::Result;
use crossbeam_channel::Sender;
use std::process;

use crate::models::NodeConfig;
use crate::models::node::{NodeDataError, NodeUpdate, NodeStateMutex};
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

/// Fetches and updates `node_state` in place.
///
/// Returns whether some update is made, and atomic updates of new state.
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
    /// Returns a tuple of whether some update is made, and atomic updates of new state.
    macro_rules! cmp_upd_state {
        ( $node_state:expr, [$( $attr:ident ),*] ) => {{
            let mut data = NodeUpdate {
                $($attr: None,)*
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
