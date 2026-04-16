use anyhow::Result;
use gethostname::gethostname;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod command;
mod exec;
mod report;
use crate::client::command::handle_command;
use crate::client::report::thread_get_report;
use crate::iosered::IOSerialized;
use crate::models::{Command, ReportState, Response};

/// Client-side main function to communicate with server.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(mut stream: TcpStream) -> Result<()> {
    // use nonblocking to reduce complexity and send keep-alive messages
    stream.set_nonblocking(true)?;

    // start thread to get report in the background
    let report_state: ReportState = Arc::new(Mutex::new((
        Err("Initializing".to_owned()),
        Err("Initializing".to_owned()),
        UNIX_EPOCH,
    )));
    {
        let report_state = Arc::clone(&report_state);
        thread::spawn(move || {
            thread_get_report(report_state);
        });
    }

    // respond hostname on new connection
    stream.write(&Response::Connect(
        gethostname()
            .to_str()
            .map(|v| v.to_owned())
            .unwrap_or(String::new()),
    ))?;

    // initialize variables for report_sync feature
    let mut report_sync = false;
    let mut report_sync_last_update = UNIX_EPOCH;
    let mut last_keepalive = SystemTime::now();

    loop {
        let mut command_opt = None;

        let mut _buf = [0u8; 4];
        match stream.peek(&mut _buf) {
            Ok(_) => command_opt = Some(stream.read::<Command>()?),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("encountered IO error: {e}"),
        };

        if let Some(command) = command_opt {
            println!("Received {command}");
            handle_command(&mut stream, command, &report_state, &mut report_sync)?;
        }

        // server should deal with report_sync reports gracefully,
        // in case a report is responded while a command is being sent
        // i.e. server should quietly update report even if it's expecting another response
        if report_sync {
            let guard = report_state.lock().unwrap();
            let (ref sessions, ref wg_peers, ref update_time) = *guard;
            // only sync if there is an update
            if *update_time > report_sync_last_update {
                stream.write(&Response::Report(sessions.clone(), wg_peers.clone()))?;
                report_sync_last_update = *update_time;
                last_keepalive = SystemTime::now();
            }
        }

        if SystemTime::now() - Duration::from_secs(60) >= last_keepalive {
            stream.write(&Response::KeepAlive)?;
            last_keepalive = SystemTime::now();
        }

        thread::sleep(Duration::from_secs(1));
    }
}
