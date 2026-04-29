use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Local;
use colored::Colorize;
use std::os::unix::net::UnixStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::hub::{ClientCommand, ClientResponse, Node};
use crate::models::node::NodeDataError;
use crate::models::packet::{Command, Response, ResultStatus};
use crate::traits::iosered::IOSerialized;
use crate::utils::get_env_var_strict;

macro_rules! match_strict {
    ( $response:expr, $pattern:pat, $return:expr ) => {
        match $response {
            $pattern => $return,
            ClientResponse::Failure(e) => {
                eprintln!("Failure: {}", e);
                std::process::exit(1);
            }
            _ => {
                panic!("invalid hub daemon response: {}", $response);
            }
        }
    };
}

/// Sends `FindNode` command to hub daemon,
/// returns `Node` if found; otherwise, fail and exit.
fn find_node(stream: &mut UnixStream, node: String) -> Result<Node> {
    stream.write(&ClientCommand::FindNode((*node).to_owned()))?;
    let resp = stream.read::<ClientResponse>()?;
    match_strict!(resp, ClientResponse::Node(node), Ok(node))
}

/// Prints node list in a human-friendly format.
fn print_node_list(nodes: Vec<Node>) -> () {
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
                    NodeDataError::NotMonitored => {} // no verbose on not monitored attributes
                    NodeDataError::Message(msg) => {
                        println!("\n{}: {msg}", $attr.yellow().bold())
                    }
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
}

/// Executes command on a specific node and streams result.
fn exec_command(stream: &mut UnixStream, node: Node, command: Command) -> Result<()> {
    let mut wait_timeout = get_env_var_strict("NODE_WAIT_TIMEOUT", Some(0));
    let expire_time = match wait_timeout {
        0 => UNIX_EPOCH,
        _ => SystemTime::now() + Duration::from_secs(wait_timeout),
    };

    println!(
        "{} ({})",
        node.address.to_string().bold().cyan(),
        node.hostname.bold().cyan()
    );
    stream.write(&ClientCommand::RawCommand(
        node.serial,
        command,
        expire_time,
    ))?;

    if wait_timeout != 0 {
        // set a read timeout if wait_timeout is enabled
        // hub would ignore command when command is expired
        // (add a little grace period here to ensure the relative read timeout
        //  is past the absolute command timeout sent to hub)
        stream.set_read_timeout(Some(
            Duration::from_secs(wait_timeout) + Duration::from_millis(100),
        ))?;
    }

    loop {
        let result = stream.read::<ClientResponse>();

        match result {
            Ok(resp) => {
                if wait_timeout != 0 {
                    // once first response is received, wait timeout no longer applies
                    stream.set_read_timeout(None)?;
                    wait_timeout = 0;
                }

                let raw_resp = match_strict!(resp, ClientResponse::RawResponse(raw_resp), raw_resp);
                // use centralized streaming response detection to ensure consistency
                let is_streaming = crate::utils::is_streaming_response(&raw_resp);
                match raw_resp {
                    Response::ResultStream(status, line) => match status {
                        ResultStatus::Pending => {
                            println!("{line}");
                        }
                        ResultStatus::Timeout => {
                            println!("{}: {}", "Done".bold(), "Timeout (Exec)".yellow().bold());
                        }
                        ResultStatus::Success => {
                            println!("{}: {}", "Done".bold(), "Success".green().bold());
                        }
                        ResultStatus::Failure => {
                            println!("{}: {}", "Done".bold(), "Failure".red().bold());
                        }
                    },
                    _ => {
                        // return immediately on invalid response
                        eprintln!("Invalid node response: {raw_resp}");
                        return Ok(());
                    }
                }
                if !is_streaming {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                println!("{}: {}", "Done".bold(), "Timeout (Wait)".yellow().bold());
                return Ok(());
            }
            Err(e) => Err(e)?,
        }
    }
}

/// Main function for command line client.
///
/// Note: This is a minimal viable implementation.
pub fn main(stream: &mut UnixStream, command: String) -> Result<()> {
    match command.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["subscribe", ..] => {
            println!("Node state atomic updates will be printed in terminal.");
            println!("NOTE: Integrations should communicate with hub over socket.");

            stream.write(&ClientCommand::Subscribe)?;
            loop {
                let resp = stream.read::<ClientResponse>()?;
                println!("{}", resp);
            }
        }
        ["list", args @ ..] => {
            stream.write(&ClientCommand::List)?;
            let resp = stream.read::<ClientResponse>()?;
            let mut nodes = match_strict!(resp, ClientResponse::List(nodes), nodes);

            if args.contains(&"sorted") {
                nodes.sort_by(|a, b| a.address.cmp(&b.address))
            }

            print_node_list(nodes);
        }
        [node, label @ ..] => {
            let command = Command::Execute(label.join(" "), true);

            if node == &"-" {
                stream.write(&ClientCommand::List)?;
                let resp = stream.read::<ClientResponse>()?;
                let nodes = match_strict!(resp, ClientResponse::List(nodes), nodes);

                for (i, node) in nodes.into_iter().filter(|node| node.connected).enumerate() {
                    if i != 0 {
                        println!("");
                    }

                    exec_command(stream, node, command.clone())?;
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
