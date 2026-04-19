use crate::models::nodestate::{Session, SessionsResult, WgPeer, WgPeersResult};
use crate::utils::exec;

/// Executes `who -w` and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_sessions() -> SessionsResult {
    let output = exec("who", ["-w"])?;

    let mut sessions = Vec::<Session>::new();

    for line in output.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(format!("Command 'who' did not produce a valid output"));
        }

        let len = parts.len();
        let mut roffset = 1;

        // NAME
        let user = parts[0].to_owned();

        // COMMENT (typically contains 'from' inside parenthesis)
        let supposed_comment = parts[len - roffset];
        let mut from = None;
        if supposed_comment.starts_with("(") && supposed_comment.ends_with(")") {
            from = Some(supposed_comment[1..supposed_comment.len() - 1].to_owned());
            roffset += 1;
        }

        // TIME (YYYY-MM-DD HH:MM)
        let login = format!("{} {}", parts[len - roffset - 1], parts[len - roffset]);

        sessions.push(Session { user, from, login });
    }

    Ok(sessions)
}

/// Executes `wg` and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_wg_peers() -> WgPeersResult {
    let output = exec("wg", [])?;

    let mut wg_peers = Vec::<WgPeer>::new();

    let mut interface = "N/A";
    let mut lines = output.lines();
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
                let mut endpoint = None;
                let mut latest_handshake = None;

                while let Some(line) = lines.next() {
                    let line_split = line.split_whitespace().collect::<Vec<_>>();
                    match line_split.as_slice() {
                        ["endpoint:", endpoint_ref, ..] => {
                            endpoint = Some((*endpoint_ref).to_owned());
                        }
                        ["latest", "handshake:", latest_handshake_ref @ ..] => {
                            latest_handshake = Some(latest_handshake_ref.join(" "));
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
