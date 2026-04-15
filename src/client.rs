use anyhow::Result;
use gethostname::gethostname;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

mod exec;
mod report;
use crate::client::report::get_report;
use crate::iosered::IOSerialized;
use crate::models::{Command, ReportState, Response};

fn thread_get_report(report_state: ReportState) -> () {
    loop {
        let (sessions, wg_peers) = get_report();

        {
            let mut guard = report_state.lock().unwrap();
            let (ref mut s, ref mut w, ref mut t) = *guard;
            if *s != sessions || *w != wg_peers {
                (*s, *w, *t) = (sessions, wg_peers, SystemTime::now());
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}

/// Client-side main function to communicate with server.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(mut stream: TcpStream) -> Result<()> {
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

        if report_sync {
            // when report_sync=true, stream is non-blocking
            // we peek to check if there is a command to be read
            let mut _buf = [0u8; 4];
            match stream.peek(&mut _buf) {
                Ok(_) => command_opt = Some(stream.read::<Command>()?),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("encountered IO error: {e}"),
            };
        } else {
            // when report_sync=false, stream is blocking
            // we read the command directly
            command_opt = Some(stream.read::<Command>()?);
        }

        if let Some(command) = command_opt {
            println!("Received {command}");

            match command {
                Command::Report => {
                    let guard = report_state.lock().unwrap();
                    let (ref sessions, ref wg_peers, _) = *guard;
                    stream.write(&Response::Report(sessions.clone(), wg_peers.clone()))?;
                }
                Command::ReportSyncStart => {
                    stream.set_nonblocking(true)?;

                    report_sync = true;
                    stream.write(&Response::ReportSync(true))?;
                }
                Command::ReportSyncStop => {
                    stream.set_nonblocking(true)?;

                    report_sync = false;
                    stream.write(&Response::ReportSync(false))?;
                }
                _ => {
                    eprintln!("Not implemented");
                    stream.write(&Response::Result(false, "Not implemented".to_owned()))?;
                }
            }
        } else {
            // command_opt may only be None when report_sync=false
            // we double check report_sync value in case more sync features are added in the future
            if report_sync {
                let guard = report_state.lock().unwrap();
                let (ref sessions, ref wg_peers, ref update_time) = *guard;
                if *update_time > report_sync_last_update {
                    stream.write(&Response::Report(sessions.clone(), wg_peers.clone()))?;
                    report_sync_last_update = *update_time;
                    last_keepalive = SystemTime::now();
                }
            }

            if SystemTime::now() - Duration::from_secs(30) >= last_keepalive {
                stream.write(&Response::KeepAlive)?;
                last_keepalive = SystemTime::now();
            }

            thread::sleep(Duration::from_secs(1));
        }
    }
}
