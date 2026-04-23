use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;

use crate::models::hub::ClientResponse;
use crate::models::nodestate::NodeStateError;
use crate::models::packet::Response;

pub fn handle_result(result: ClientResponse) -> Result<()> {
    match result {
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
        ClientResponse::RawResponse(response) => {
            match response {
                Response::Result(success, message) => {
                    if success {
                        if message == "" {
                            println!("Success (no message)");
                        } else {
                            println!("{message}");
                        }
                    } else {
                        if message == "" {
                            eprintln!("Error (no message)");
                        } else {
                            eprintln!("{message}");
                        }
                    }
                }
                _ => {
                    println!("{response}");
                }
            }
            Ok(())
        }
        ClientResponse::Failure(error) => Err(anyhow!("Failure: {error}")),
        _ => Err(anyhow!("Received invalid response from hub: {result}")),
    }
}
