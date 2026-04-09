use std::io::Result;
use std::net::TcpStream;

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
            Command::Report => {}
            _ => {
                eprintln!("Not implemented.");
                stream.write(&Message::Result(false, "Not implemented.".to_owned()))?;
            }
        }
    }
}
