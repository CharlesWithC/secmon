use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;

use crate::models::hub::ClientCommand;
use crate::traits::iosered::IOSerialized;

mod handler;

/// Client main function for handling local client command.
///
/// The command is read from command line arguments.
pub fn main(socket_path: String, command: String) -> Result<()> {
    match UnixStream::connect(socket_path) {
        Ok(ref mut stream) => {
            let result = handler::handle_command(stream, command);
            stream.write(&ClientCommand::Quit)?; // quit to close connection gracefully
            result // propaget result
        }
        Err(e) => Err(anyhow!(
            "Unable to connect to hub daemon; Is hub daemon running?\n{e}"
        )),
    }
}
