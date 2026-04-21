use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;
use std::os::unix::net::UnixStream;

use crate::iosered::IOSerialized;
use crate::models::hub::{CtrlCmd, CtrlRes};

/// Control main function for handling local control command.
///
/// The command is read from command line arguments.
///
/// Currently, this is a non-blocking function.
///
/// In the future, interactive sessions may be supported,
/// which would make this function blocking.
pub fn main(socket_path: String, command: String, _args: Vec<String>) -> Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;

    match command.as_str() {
        "list" => {
            stream.write(&CtrlCmd::List)?;

            let result = stream.read::<CtrlRes>()?;
            match result {
                CtrlRes::List(nodes) => {
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
                } // _ => Err(anyhow!("Received invalid response from hub: {result}")),
            }
        }
        _ => Err(anyhow!("Unknown command; Use 'secmon help' for help")),
    }
}
