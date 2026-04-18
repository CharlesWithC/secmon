use anyhow::Result;
use std::net::TcpStream;
use std::time::SystemTime;

use crate::exec::exec;
use crate::iosered::IOSerialized;
use crate::models::node::NodeState;
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
    state_sync: &mut bool,
) -> Result<()> {
    match command {
        Command::KeepAlive => {
            stream.write(&Response::KeepAlive)?;
        }
        Command::NodeState => {
            let guard = node_state.lock().unwrap();
            let (ref sessions, ref wg_peers, _) = *guard;
            stream.write(&Response::NodeState(sessions.clone(), wg_peers.clone()))?;
        }
        Command::StateSyncStart => {
            *state_sync = true;
            stream.write(&Response::StateSync(true))?;
        }
        Command::StateSyncStop => {
            *state_sync = false;
            stream.write(&Response::StateSync(false))?;
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

/// Fetches state and updates `node_state`.
pub fn update_node_state(node_state: &NodeState) -> () {
    let sessions = get_sessions();
    let wg_peers = get_wg_peers();

    {
        let mut guard = node_state.lock().unwrap();
        let (ref mut s, ref mut w, ref mut t) = *guard;
        if *s != sessions || *w != wg_peers {
            (*s, *w, *t) = (sessions, wg_peers, SystemTime::now());
        }
    }
}
