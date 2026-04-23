use anyhow::Result;
use std::os::unix::net::UnixStream;

use crate::models::hub::ClientCommand;
use crate::traits::iosered::IOSerialized;

mod handler;

/// Client main function for handling local client command.
///
/// The command is read from command line arguments.
pub fn main(socket_path: String, command: String) -> Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;

    let result = handler::handle_command(&mut stream, command);
    stream.write(&ClientCommand::Quit)?; // quit to close connection gracefully
    result // propaget result
}
