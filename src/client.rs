use anyhow::Result;
use gethostname::gethostname;
use std::net::TcpStream;

mod exec;
mod report;
use crate::client::report::get_report;
use crate::iosered::IOSerialized;
use crate::models::{Command, Response};

/// Client-side main function to communicate with server.
///
/// This is a blocking function and does not exit unless interrupted.
pub fn main(mut stream: TcpStream) -> Result<()> {
    stream.write(&Response::Connect(
        gethostname()
            .to_str()
            .map(|v| v.to_owned())
            .unwrap_or(String::new()),
    ))?;

    loop {
        let command = stream.read::<Command>()?;
        println!("Received {}", command);

        match command {
            Command::Report => {
                let (sessions, wg_peers) = get_report();
                stream.write(&Response::Report(sessions, wg_peers))?;
            }
            _ => {
                eprintln!("Not implemented");
                stream.write(&Response::Result(false, "not implemented".to_owned()))?;
            }
        }
    }
}
