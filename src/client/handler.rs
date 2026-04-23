use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;

use crate::models::hub::ClientResponse;
use crate::models::packet::Response;

pub fn handle_result(result: ClientResponse) -> Result<()> {
    match result {
        ClientResponse::List(nodes) => {
            for (i, node) in nodes.iter().enumerate() {
                if i != 0 {
                    println!("");
                }

                println!("{}: {}", "node".green().bold(), node.hostname.green());
                println!("  {}: {}", "address".bold(), node.address);
                let last_state_update_dt: DateTime<Local> = node.last_state_update.into();
                println!(
                    "  {}: {}",
                    "last state update".bold(),
                    last_state_update_dt.format("%F %T")
                );
                println!(
                    "  {}? {}",
                    "connected".bold(),
                    if node.connected {
                        "yes".green()
                    } else {
                        "no".red()
                    }
                );

                match &node.sessions {
                    Ok(sessions) => sessions.into_iter().for_each(|session| {
                        println!("");
                        println!("{}: {}", "session".yellow().bold(), session.user);
                        if let Some(from) = &session.from {
                            println!("  {}: {}", "from".bold(), from);
                        }

                        let dt: DateTime<Local> = session.login.into();
                        let parsed: String = format!("{}", dt.format("%F %T"));
                        println!("  {}: {}", "login".bold(), parsed);
                    }),
                    Err(e) => {
                        println!("");
                        println!("{}: {}", "sessions".yellow().bold(), e);
                    }
                }

                match &node.wg_peers {
                    Ok(wg_peers) => wg_peers.into_iter().for_each(|wg_peer| {
                        println!("");
                        println!("{}: {}", "wg peer".yellow().bold(), wg_peer.peer);
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
                    Err(e) => {
                        println!("");
                        println!("{}: {}", "wg peers".yellow().bold(), e);
                    }
                }
            }
            Ok(())
        }
        ClientResponse::RawResponse(response) => {
            match response {
                Response::Result(success, message) => {
                    if success {
                        if message == "" {
                            println!("Command succeeded with no message");
                        } else {
                            println!("{message}");
                        }
                    } else {
                        if message == "" {
                            eprintln!("Command failed with no message");
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
