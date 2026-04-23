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
    let resp = stream.read::<ClientResponse>()?;
    match resp {
        ClientResponse::Failure(error) => Err(anyhow!("Failure: {error}")),
        ClientResponse::Node(node) => Ok(node),
        _ => {
            return Err(anyhow!("Hub daemon provided an invalid response: {resp}"));
        }
    }
}

/// Executes command on given nodes and handles response.
///
/// Closes connection with hub once done.
fn exec_node_cmd(stream: &mut UnixStream, node: &str, command: Command) -> Result<()> {
    if node == "-" {
        // send command to all connected nodes
        stream.write(&ClientCommand::List)?;
        let resp = stream.read::<ClientResponse>()?;
        match resp {
            ClientResponse::List(nodes) => {
                let mut is_first = true;
                for node in nodes {
                    if node.connected {
                        if !is_first {
                            println!("");
                        }
                        is_first = false;

                        println!("{} ({}) responded:", node.address, node.hostname);

                        stream.write(&ClientCommand::RawCommand(node.serial, command.clone()))?;

                        let result = stream.read::<ClientResponse>()?;
                        handler::handle_result(result)?;
                    }
                }
            }
            _ => {
                return Err(anyhow!("Hub daemon provided an invalid response: {resp}"));
            }
        }
    } else {
        // send command to a specific node
        let node = find_node(stream, node.to_owned())?;
        stream.write(&ClientCommand::RawCommand(node.serial, command))?;

        let result = stream.read::<ClientResponse>()?;
        handler::handle_result(result)?;
    }

    stream.write(&ClientCommand::Quit)?;

    Ok(())
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

            exec_node_cmd(
                &mut stream,
                *node,
                Command::Service(mode, flag_now, services),
            )?;
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

            exec_node_cmd(&mut stream, *node, Command::Reboot(minutes))?;
        }
        [node, "shutdown-cancel"] => {
            exec_node_cmd(&mut stream, *node, Command::ShutdownCancel)?;
        }
        _ => Err(anyhow!("Invalid command; Use 'secmon help' for help"))?,
    }

    Ok(())
}
