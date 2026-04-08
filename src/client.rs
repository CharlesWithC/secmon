use std::io::Result;
use std::net::TcpStream;

use crate::comm::SendRecv;
use crate::models::Command;

pub fn comm_server(mut stream: TcpStream) -> Result<()> {
    loop {
        let command = stream.recv::<Command>()?;
        println!("Received command: {:?}", command);
    }
}
