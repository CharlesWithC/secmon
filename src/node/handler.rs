use anyhow::Result;
use std::net::TcpStream;
use std::process;

use crate::exec::Exec;
use crate::iosered::IOSerialized;
use crate::models::NodeConfig;
use crate::models::nodestate::{NodeState, NodeStateDiff};
use crate::models::packet::{Command, Response, ServiceMode};
use crate::node::state::{get_sessions, get_wg_peers};

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
/// Returns whether an update is made, and the difference between new and old node state.
pub fn update_node_state(
    node_config: NodeConfig,
    node_state: &mut NodeState,
) -> (bool, NodeStateDiff) {
    let sessions = if node_config.enable_sessions {
        Some(get_sessions())
    } else {
        None
    };

    let wg_peers = if node_config.enable_wg_peers {
        Some(get_wg_peers())
    } else {
        None
    };

    let mut diff = NodeStateDiff {
        sessions: None,
        wg_peers: None,
    };
    let mut updated = false;
    if sessions != (*node_state).sessions {
        (*node_state).sessions = sessions.clone();
        diff.sessions = Some(sessions);
        updated = true;
    }
    if wg_peers != (*node_state).wg_peers {
        (*node_state).wg_peers = wg_peers.clone();
        diff.wg_peers = Some(wg_peers);
        updated = true;
    }

    (updated, diff)
}
