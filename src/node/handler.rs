use anyhow::Result;
use std::net::TcpStream;
use std::process;

use crate::models::NodeConfig;
use crate::models::nodestate::{NodeState, NodeStateDiff, NodeStateError};
use crate::models::packet::{Command, Response, ServiceMode};
use crate::node::state::{get_sessions, get_wg_peers};
use crate::traits::exec::Exec;
use crate::traits::iosered::IOSerialized;

/// Returns `Response::Result` constructed from `Result`.
fn response_result(result: Result<String, String>) -> Response {
    match result {
        Ok(output) => Response::Result(true, output),
        Err(error) => Response::Result(false, error),
    }
}

/// Handles command from hub.
pub fn handle_command(
    stream: &mut TcpStream,
    command: &Command,
    node_state: &NodeState,
) -> Result<()> {
    match command {
        Command::NodeState => {
            stream.write(&Response::NodeState(node_state.clone()))?;
        }
        Command::Service(mode, now, services) => {
            let mode = match mode {
                ServiceMode::Enable => "enable",
                ServiceMode::Disable => "disable",
            };

            let args: Vec<&str> = [mode]
                .into_iter()
                .chain(now.then_some("--now"))
                .chain(services.into_iter().map(|v| v.as_str()))
                .collect();

            let mut command = process::Command::new("systemctl");
            command.args(args);
            stream.write(&response_result(command.run()))?;
        }
        Command::Reboot(minutes) => {
            let minutes_arg = format!("+{minutes}");

            let args: Vec<&str> = ["-r"]
                .into_iter()
                .chain(Some(minutes_arg.as_str()))
                .collect();

            let mut command = process::Command::new("shutdown");
            command.args(args);
            stream.write(&response_result(command.run()))?;
        }
        Command::ShutdownCancel => {
            let mut command = process::Command::new("shutdown");
            command.args(["-c"]);
            stream.write(&response_result(command.run()))?;
        }
    };

    Ok(())
}

/// Fetches and updates `node_state` in place.
///
/// Returns whether some update is made, and the difference between new and old node states.
pub fn update_node_state(
    node_config: NodeConfig,
    node_state: &mut NodeState,
) -> (bool, NodeStateDiff) {
    /// Macro to fetch updates based on `node_config`.
    macro_rules! fetch_updates {
        ( $( $attr:ident ),* ) => {
            paste::paste! {
            $(let $attr = if node_config.[<enable_ $attr>] {
                let result = [<get_ $attr>]();
                match result {
                    // repack the result to use NodeStateError
                    Ok(result) => Ok(result),
                    Err(e) => Err(NodeStateError::Message(e)),
                }
            } else {
                Err(NodeStateError::NotMonitored)
            };)*
        }}
    }
    fetch_updates!(sessions, wg_peers);

    /// Macro to compare and update node state.
    ///
    /// Returns a tuple of whether some update is made, and the difference between new and old states.
    macro_rules! cmp_upd_state {
        ( $node_state:expr, [$( $attr:ident ),*] ) => {{
            let mut diff = NodeStateDiff {
                $($attr: None,)*
            };
            let mut updated = false;
            $(if $attr != $node_state.$attr {
                $node_state.$attr = $attr.clone();
                diff.$attr = Some($attr);
                updated = true;
            })*
            (updated, diff)
        }}
    }

    cmp_upd_state!(node_state, [sessions, wg_peers])
}
