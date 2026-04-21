use std::time::{Duration, UNIX_EPOCH};

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
            return Err(format!("command 'who' did not produce a valid output"));
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

/// Executes `wg` twice and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_wg_peers() -> WgPeersResult {
    // `wg show` only supports one attribute in each run
    // the output format is (interface, peer, <attribute>)
    // thus, we run `wg show` twice with different arguments

    let mut wg_peers = Vec::<WgPeer>::new();

    let output_endpoints = exec("wg", ["show", "all", "endpoints"])?;
    for line in output_endpoints.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(format!("command 'wg' did not produce a valid output"));
        }

        wg_peers.push(WgPeer {
            interface: parts[0].to_owned(),
            peer: parts[1].to_owned(),
            endpoint: if parts[2] != "(none)" {
                Some(parts[2].to_owned())
            } else {
                None
            },
            latest_handshake: None,
        })
    }

    let output_handshake = exec("wg", ["show", "all", "latest-handshakes"])?;
    for line in output_handshake.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(format!("command 'wg' did not produce a valid output"));
        }

        wg_peers.iter_mut().for_each(|wg_peer| {
            if wg_peer.peer.as_str() == parts[1] {
                let timestamp_res = parts[2].parse::<u64>();
                if let Ok(timestamp) = timestamp_res
                    && timestamp != 0
                {
                    wg_peer.latest_handshake = Some(UNIX_EPOCH + Duration::from_secs(timestamp));
                }
            }
        })
    }

    Ok(wg_peers)
}
