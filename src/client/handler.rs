use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;
use std::os::unix::net::UnixStream;

use crate::models::hub::{ClientCommand, ClientResponse};
use crate::models::node::Node;
use crate::models::nodestate::NodeStateError;
use crate::models::packet::{Command, Response};
use crate::traits::iosered::IOSerialized;

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
fn exec_command(stream: &mut UnixStream, node: &str, command: Command) -> Result<()> {
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

                        println!(
                            "{} ({})",
                            node.address.to_string().bold().cyan(),
                            node.hostname.bold().cyan()
                        );
                        stream.write(&ClientCommand::RawCommand(node.serial, command.clone()))?;

                        let resp = stream.read::<ClientResponse>()?;
                        handle_response(resp)?;
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

        println!(
            "{} ({})",
            node.address.to_string().bold().cyan(),
            node.hostname.bold().cyan()
        );
        stream.write(&ClientCommand::RawCommand(node.serial, command))?;

        let resp = stream.read::<ClientResponse>()?;
        handle_response(resp)?;
    }

    Ok(())
}

/// Handles response from hub.
///
/// That is, parses the response and prints it to terminal.
fn handle_response(resp: ClientResponse) -> Result<()> {
    match resp {
        ClientResponse::List(nodes) => {
            for (i, node) in nodes.iter().enumerate() {
                if i != 0 {
                    println!("");
                }

                println!(
                    "{}: {} ({})",
                    "node".green().bold(),
                    node.hostname.green(),
                    if node.connected {
                        "connected".green()
                    } else {
                        "disconnected".red()
                    }
                );
                println!("  {}: {}", "serial".bold(), node.serial);
                println!("  {}: {}", "address".bold(), node.address);
                let last_state_update_dt: DateTime<Local> = node.last_state_update.into();
                println!(
                    "  {}: {}",
                    "last state update".bold(),
                    last_state_update_dt.format("%F %T")
                );

                macro_rules! print_err {
                    ( $err:expr, $attr:expr ) => {
                        match $err {
                            NodeStateError::Initializing => {
                                println!("\n{}: Initializing", $attr.yellow().bold())
                            }
                            NodeStateError::Message(msg) => {
                                println!("\n{}: {msg}", $attr.yellow().bold())
                            }
                            _ => {}
                        }
                    };
                }

                match &node.sessions {
                    Ok(sessions) => {
                        if sessions.len() > 0 {
                            println!("\n{}:", "sessions".yellow().bold());
                            let max_user_len = sessions
                                .into_iter()
                                .map(|session| session.user.len())
                                .max()
                                .unwrap_or(0);
                            sessions.into_iter().for_each(|session| {
                                let dt: DateTime<Local> = session.login.into();
                                let from = if let Some(from) = &session.from {
                                    format!("({from})")
                                } else {
                                    format!("(/)")
                                };
                                println!(
                                    "  {user: <user_width$}{login: <7}{from}",
                                    user = session.user,
                                    user_width = max_user_len + 2,
                                    login = dt.format("%H:%M"),
                                    from = from
                                );
                            })
                        }
                    }
                    Err(e) => print_err!(e, "sessions"),
                }

                match &node.wg_peers {
                    Ok(wg_peers) => wg_peers.into_iter().for_each(|wg_peer| {
                        println!("\n{}: {}", "wg peer".yellow().bold(), wg_peer.peer);
                        println!("  {}: {}", "interface".bold(), wg_peer.interface);
                        if let Some(endpoint) = &wg_peer.endpoint {
                            println!("  {}: {}", "endpoint".bold(), endpoint);
                        }
                        if let Some(latest_handshake) = &wg_peer.latest_handshake {
                            let dt: DateTime<Local> = (*latest_handshake).into();
                            let parsed = format!("{}", dt.format("%F %T"));

                            println!("  {}: {}", "latest handshake".bold(), parsed);
                        }
                    }),
                    Err(e) => print_err!(e, "wg peers"),
                }
            }
            Ok(())
        }
        ClientResponse::NodeStateDiff(..) => {
            // minimal viable handling
            // command-line subscribe is for debug purpose anyway
            println!("{}", resp);
            Ok(())
        }
        ClientResponse::RawResponse(response) => {
            match response {
                Response::Result(success, message) => {
                    println!(
                        "{}: {}",
                        if success {
                            "Success".green().bold()
                        } else {
                            "Error".red().bold()
                        },
                        if message != "" {
                            message
                        } else {
                            "No message".italic().to_string()
                        }
                    );
                }
                _ => {
                    println!("{response}");
                }
            }
            Ok(())
        }
        ClientResponse::Failure(error) => Err(anyhow!("Failure: {error}")),
        _ => Err(anyhow!("Received invalid response from hub: {resp}")),
    }
}

/// Handles command to be sent to hub, and the corresponding response.
pub fn handle_command(stream: &mut UnixStream, command: String) -> Result<()> {
    match command.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["list", args @ ..] => {
            stream.write(&ClientCommand::List)?;

            let resp = stream.read::<ClientResponse>()?;
            match resp {
                ClientResponse::List(mut nodes) => {
                    if args.contains(&"sorted") {
                        nodes.sort_by(|a, b| a.address.cmp(&b.address))
                    }

                    handle_response(ClientResponse::List(nodes))?;
                }
                _ => {
                    return Err(anyhow!("Hub daemon provided an invalid response: {resp}"));
                }
            }
        }
        ["subscribe", ..] => {
            stream.write(&ClientCommand::Subscribe)?;

            println!("Node state diff updates will be printed in terminal.");
            println!("NOTE: Integrations should communicate with hub over socket.");

            loop {
                let resp = stream.read::<ClientResponse>()?;
                handle_response(resp)?;
            }
        }
        [node, "execute", label @ ..] => {
            exec_command(stream, *node, Command::Execute(label.join(" ")))?;
        }
        _ => Err(anyhow!("Invalid command; Use 'secmon help' for help"))?,
    }

    Ok(())
}
