use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;
use std::os::unix::net::UnixStream;

use crate::models::hub::{ClientCommand, ClientResponse, Node};
use crate::models::node::NodeDataError;
use crate::models::packet::{Command, Response, ResultStatus};
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

/// Executes command on a specific node and handles response.
fn exec_command(stream: &mut UnixStream, node: Node, command: Command) -> Result<()> {
    println!(
        "{} ({})",
        node.address.to_string().bold().cyan(),
        node.hostname.bold().cyan()
    );
    stream.write(&ClientCommand::RawCommand(node.serial, command))?;

    loop {
        let resp = stream.read::<ClientResponse>()?;
        match resp {
            ClientResponse::RawResponse(resp) => match resp {
                Response::ResultStream(status, line) => match status {
                    ResultStatus::Pending => {
                        println!("{line}");
                    }
                    ResultStatus::Success => {
                        println!("Done: {}", "Success".green().bold());
                        return Ok(());
                    }
                    ResultStatus::Failure => {
                        println!("Done: {}", "Failure".red().bold());
                        return Ok(());
                    }
                },
                _ => handle_response(ClientResponse::RawResponse(resp))?,
            },
            _ => handle_response(resp)?,
        }
    }
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
                            NodeDataError::Initializing => {
                                println!("\n{}: Initializing", $attr.yellow().bold())
                            }
                            NodeDataError::Message(msg) => {
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
        ClientResponse::NodeUpdate(..) => {
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

/// Main function for command line client.
///
/// This sends command to hub, and processes response from hub.
pub fn main(stream: &mut UnixStream, command: String) -> Result<()> {
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

            println!("Node state atomic updates will be printed in terminal.");
            println!("NOTE: Integrations should communicate with hub over socket.");

            loop {
                let resp = stream.read::<ClientResponse>()?;
                handle_response(resp)?;
            }
        }
        [node, "execute", label @ ..] => {
            let command = Command::Execute(label.join(" "), true);

            if node == &"-" {
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

                                exec_command(stream, node, command.clone())?;
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!("Hub daemon provided an invalid response: {resp}"));
                    }
                }
            } else {
                let node = find_node(stream, node.to_string())?;
                exec_command(stream, node, command)?;
            }
        }
        _ => Err(anyhow!("Invalid command; Use 'secmon help' for help"))?,
    }

    Ok(())
}
