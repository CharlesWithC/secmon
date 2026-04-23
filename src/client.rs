use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;

use crate::models::hub::{ClientCommand, ClientResponse};
use crate::models::node::Node;
use crate::models::packet::{Command, ServiceMode};
use crate::traits::iosered::IOSerialized;

mod handler;

/// Sends `FindNode` command to hub daemon,
/// returns `Node` if found; otherwise, raises an error.
fn find_node(stream: &mut UnixStream, node: String) -> Result<Node> {
    stream.write(&ClientCommand::FindNode((*node).to_owned()))?;
    let node_res = stream.read::<ClientResponse>()?;
    match node_res {
        ClientResponse::Failure(error) => Err(anyhow!("Failure: {error}")),
        ClientResponse::Node(node) => Ok(node),
        _ => {
            return Err(anyhow!(
                "Hub daemon provided an invalid response: {node_res}"
            ));
        }
    }
}

/// Client main function for handling local client command.
///
/// The command is read from command line arguments.
///
/// Currently, this is a non-blocking function.
///
/// In the future, interactive sessions may be supported,
/// which would make this function blocking.
pub fn main(socket_path: String, command: String) -> Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;

    match command.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["list", ..] => {
            stream.write(&ClientCommand::List)?;

            let result = stream.read::<ClientResponse>()?;
            stream.write(&ClientCommand::Quit)?;
            handler::handle_result(result)?;
        }
        [node, "service", mode @ ("enable" | "disable"), args @ ..] => {
            let mode = if mode == &"enable" {
                ServiceMode::Enable
            } else {
                ServiceMode::Disable
            };

            let flag_now = args.contains(&"--now");
            let has_more_flags = args.iter().any(|v| v.starts_with("-") && v != &"--now");
            if has_more_flags {
                return Err(anyhow!("Only --now flag is allowed"));
            }

            let services = args
                .iter()
                .filter(|v| !v.starts_with("--"))
                .map(|v| (*v).to_owned())
                .collect::<Vec<String>>();
            if services.len() == 0 {
                return Err(anyhow!("At least one service must be specified"));
            }

            let node = find_node(&mut stream, (*node).to_owned())?;
            stream.write(&ClientCommand::RawCommand(
                node.serial,
                Command::Service(mode, flag_now, services),
            ))?;

            let result = stream.read::<ClientResponse>()?;
            stream.write(&ClientCommand::Quit)?;
            handler::handle_result(result)?;
        }
        [node, "reboot", args @ ..] => {
            let minutes = args
                .iter()
                .find(|v| v.starts_with("+"))
                .map(|v| {
                    v.parse::<u32>()
                        .map_err(|e| anyhow!("Unable to parse \"+<minutes>\": {e}"))
                })
                .ok_or(anyhow!("\"+<minutes>\" must be provided"))??;

            let node = find_node(&mut stream, (*node).to_owned())?;
            stream.write(&ClientCommand::RawCommand(
                node.serial,
                Command::Reboot(minutes),
            ))?;

            let result = stream.read::<ClientResponse>()?;
            stream.write(&ClientCommand::Quit)?;
            handler::handle_result(result)?;
        }
        [node, "shutdown-cancel"] => {
            let node = find_node(&mut stream, (*node).to_owned())?;
            stream.write(&ClientCommand::RawCommand(
                node.serial,
                Command::ShutdownCancel,
            ))?;

            let result = stream.read::<ClientResponse>()?;
            stream.write(&ClientCommand::Quit)?;
            handler::handle_result(result)?;
        }
        _ => Err(anyhow!("Invalid command; Use 'secmon help' for help"))?,
    }

    Ok(())
}
