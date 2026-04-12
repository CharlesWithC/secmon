use anyhow::{Result, anyhow};
use std::process::Command;

use crate::models::{Session, WgPeer};

/// Returns a list of user sessions based on `who` command output.
pub fn get_sessions() -> Result<Vec<Session>> {
    let output = Command::new("who").args(["-w"]).output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "command 'who' did not succeed: {}",
            str::from_utf8(&output.stderr)?
        ));
    }

    let mut sessions = Vec::<Session>::new();

    for line in str::from_utf8(&output.stdout)?.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(anyhow!("command 'who' did not produce a valid output"));
        }

        let len = parts.len();
        let mut roffset = 1;

        // NAME
        let user = parts[0].to_owned();

        // COMMENT (typically contains 'from' inside parenthesis)
        let supposed_comment = parts[len - roffset];
        let mut from = String::from("N/A");
        if supposed_comment.starts_with("(") && supposed_comment.ends_with(")") {
            from = supposed_comment[1..supposed_comment.len() - 1].to_owned();
            roffset += 1;
        }

        // TIME (YYYY-MM-DD HH:MM)
        let login = format!("{} {}", parts[len - roffset - 1], parts[len - roffset]);

        sessions.push(Session { user, from, login });
    }

    Ok(sessions)
}

/// Returns a list of wireguard peers based on `wg` command output.
pub fn get_wg_peers() -> Result<Vec<WgPeer>> {
    let output = Command::new("wg").output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "command 'wg' did not succeed: {}",
            str::from_utf8(&output.stderr)?
        ));
    }

    let mut wg_peers = Vec::<WgPeer>::new();

    let mut interface = "N/A";
    let mut lines = str::from_utf8(&output.stdout)?.lines();
    while let Some(line) = lines.next() {
        let line_split = line.split_whitespace().collect::<Vec<_>>();
        match line_split.as_slice() {
            // handles interface
            ["interface:", interface_ref, ..] => {
                interface = *interface_ref;

                while let Some(line) = lines.next() {
                    let line_split = line.split_whitespace().collect::<Vec<_>>();
                    match line_split.as_slice() {
                        [] => break,   // empty line, interface block finished
                        _ => continue, // entry that we don't care
                    }
                }
            }
            // handles peer
            ["peer:", peer_ref, ..] => {
                let peer = (*peer_ref).to_owned();
                let mut endpoint = String::from("N/A");
                let mut latest_handshake = String::from("N/A");

                while let Some(line) = lines.next() {
                    let line_split = line.split_whitespace().collect::<Vec<_>>();
                    match line_split.as_slice() {
                        ["endpoint:", endpoint_ref, ..] => {
                            endpoint = (*endpoint_ref).to_owned();
                        }
                        ["latest", "handshake:", latest_handshake_ref @ ..] => {
                            latest_handshake = latest_handshake_ref.join(" ");
                        }
                        [] => break,   // empty line, peer block finished
                        _ => continue, // entry that we don't care
                    }
                }

                wg_peers.push(WgPeer {
                    interface: interface.to_owned(),
                    peer,
                    endpoint,
                    latest_handshake,
                });
            }
            // don't care other sections
            _ => continue,
        }
    }

    Ok(wg_peers)
}

/// Returns a tuple of user sessions and wireguard peers.
pub fn get_report() -> Result<(Vec<Session>, Vec<WgPeer>)> {
    let sessions = get_sessions()?;
    let wg_peers = get_wg_peers()?;

    Ok((sessions, wg_peers))
}
