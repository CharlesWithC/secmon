use anyhow::Result;
use std::net::TcpStream;

use crate::utils::exec;
use crate::iosered::IOSerialized;
use crate::models::NodeConfig;
use crate::models::nodestate::NodeState;
use crate::models::packet::{Command, Response};
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
            let (sessions, wg_peers) = node_state;
            stream.write(&Response::NodeState(sessions.clone(), wg_peers.clone()))?;
        }
        Command::ServiceEnable(now, service) | Command::ServiceDisable(now, service) => {
            let mode = match command {
                Command::ServiceEnable(..) => "enable",
                Command::ServiceDisable(..) => "disable",
                _ => panic!("should not happen"),
            };

            let args: Vec<&str> = [mode]
                .into_iter()
                .chain(now.then_some("--now"))
                .chain(Some(service.as_str()))
                .collect();

            let result = exec("systemctl", args);
            stream.write(&response_result(result))?;
        }
        Command::Reboot(minutes) => {
            let minutes_arg = format!("+{minutes}");

            let args: Vec<&str> = ["-r"]
                .into_iter()
                .chain(Some(minutes_arg.as_str()))
                .collect();

            let result = exec("shutdown", args);
            stream.write(&response_result(result))?;
        }
        Command::RebootCancel => {
            let result = exec("shutdown", ["-c"]);
            stream.write(&response_result(result))?;
        }
    };

    Ok(())
}

/// Fetches and updates `node_state` in place.
///
/// If some update is made, returns `true`; otherwise, `false`.
pub fn update_node_state(node_config: NodeConfig, node_state: &mut NodeState) -> bool {
    let sessions = if node_config.enable_sessions {
        get_sessions()
    } else {
        Err("not monitored".to_owned())
    };

    let wg_peers = if node_config.enable_wg_peers {
        get_wg_peers()
    } else {
        Err("not monitored".to_owned())
    };

    let new_node_state = (sessions, wg_peers);
    if *node_state != new_node_state {
        *node_state = new_node_state;
        true
    } else {
        false
    }
}
