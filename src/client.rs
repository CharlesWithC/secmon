use std::io::Result;
use std::net::TcpStream;

mod report;
use crate::client::report::get_report;
use crate::iosered::IOSerialized;
use crate::models::{Command, Message};

/// Client-side main function to communicate with server.
///
/// This is a blocking function and does not exit until connection is closed.
pub fn comm_server(mut stream: TcpStream) -> Result<()> {
    loop {
        let command = stream.read::<Command>()?;
        println!("Received {:?}", command);

        match command {
            Command::Report => {
                let report = get_report();
                match report {
                    Ok((sessions, wg_peers)) => {
                        stream.write(&Message::Report(sessions, wg_peers))?;
                    }
                    Err(error) => {
                        stream.write(&Message::Result(false, format!("{:?}", error)))?;
                    }
                }
            }
            _ => {
                eprintln!("Not implemented");
                stream.write(&Message::Result(false, "not implemented".to_owned()))?;
            }
        }
    }
}
