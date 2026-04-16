use anyhow::Result;
use std::net::TcpStream;

use crate::client::exec::exec;
use crate::iosered::IOSerialized;
use crate::models::{Command, ReportState, Response};

/// Returns `Response::Result` constructed from `Result`.
fn response_result(result: Result<String, String>) -> Response {
    match result {
        Ok(output) => Response::Result(true, output),
        Err(error) => Response::Result(false, error),
    }
}

/// Handles command from server.
pub fn handle_command(
    stream: &mut TcpStream,
    command: &Command,
    report_state: &ReportState,
    report_sync: &mut bool,
) -> Result<()> {
    match command {
        Command::KeepAlive => {
            stream.write(&Response::KeepAlive)?;
        }
        Command::Report => {
            let guard = report_state.lock().unwrap();
            let (ref sessions, ref wg_peers, _) = *guard;
            stream.write(&Response::Report(sessions.clone(), wg_peers.clone()))?;
        }
        Command::ReportSyncStart => {
            *report_sync = true;
            stream.write(&Response::ReportSync(true))?;
        }
        Command::ReportSyncStop => {
            *report_sync = false;
            stream.write(&Response::ReportSync(false))?;
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
