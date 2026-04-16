use anyhow::Result;
use std::net::TcpStream;

use crate::iosered::IOSerialized;
use crate::models::{Command, ReportState, Response};

/// Processes command from server.
pub fn handle_command(
    stream: &mut TcpStream,
    command: Command,
    report_state: &ReportState,
    report_sync: &mut bool,
) -> Result<()> {
    match command {
        Command::Report => {
            let guard = report_state.lock().unwrap();
            let (ref sessions, ref wg_peers, _) = *guard;
            stream.write(&Response::Report(sessions.clone(), wg_peers.clone()))?;
        }
        Command::ReportSyncStart => {
            stream.set_nonblocking(true)?;

            *report_sync = true;
            stream.write(&Response::ReportSync(true))?;
        }
        Command::ReportSyncStop => {
            stream.set_nonblocking(true)?;

            *report_sync = false;
            stream.write(&Response::ReportSync(false))?;
        }
        _ => {
            eprintln!("Not implemented");
            stream.write(&Response::Result(false, "Not implemented".to_owned()))?;
        }
    };

    Ok(())
}
