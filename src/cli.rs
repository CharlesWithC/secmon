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
                    nodes.into_iter().for_each(|node| {
                        let last_state_update_dt: DateTime<Local> = node.last_state_update.into();

                        println!("{}: {}", "node".green().bold(), node.hostname.green());
                        println!("  {}: {}", "address".bold(), node.address);
                        println!(
                            "  {}: {}",
                            "last state update".bold(),
                            last_state_update_dt.format("%F %T")
                        );

                        match node.sessions {
                            Ok(sessions) => sessions.into_iter().for_each(|session| {
                                println!("");
                                println!("{}: {}", "session".yellow().bold(), session.user);
                                if let Some(from) = session.from {
                                    println!("  {}: {}", "from".bold(), from);
                                }
                                println!("  {}: {}", "login".bold(), session.login);
                            }),
                            Err(e) => {
                                println!("");
                                println!("{}: {}", "sessions".yellow(), e);
                            }
                        }

                        match node.wg_peers {
                            Ok(wg_peers) => wg_peers.into_iter().for_each(|wg_peer| {
                                println!("");
                                println!("{}: {}", "wg peer".yellow().bold(), wg_peer.peer);
                                println!("  {}: {}", "interface".bold(), wg_peer.interface);
                                if let Some(endpoint) = wg_peer.endpoint {
                                    println!("  {}: {}", "endpoint".bold(), endpoint);
                                }
                                if let Some(latest_handshake) = wg_peer.latest_handshake {
                                    println!(
                                        "  {}: {}",
                                        "latest handshake".bold(),
                                        latest_handshake
                                    );
                                }
                            }),
                            Err(e) => {
                                println!("");
                                println!("{}: {}", "wg peers".yellow(), e);
                            }
                        }
                    });
                    Ok(())
                } // _ => Err(anyhow!("Received invalid response from hub: {result}")),
            }
        }
        _ => Err(anyhow!("Unknown command; Use 'secmon help' for help")),
    }
}
