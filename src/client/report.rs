use anyhow::{Result, anyhow};
use std::process::Command;

use crate::models::{Session, WgPeer};

/// Returns a list of user sessions based on `w` command output.
pub fn get_sessions() -> Result<Vec<Session>> {
    let output = Command::new("w").args(["-h", "-f"]).output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "command 'w' did not succeed: {}",
            str::from_utf8(&output.stderr)?
        ));
    }

    let sessions = Vec::<Session>::new();

    // for line in str::from_utf8(&output.stdout)?.lines() {
    //     let parts = line.split("\t").collect::<Vec<_>>();
    //     if parts.len() < 8 {
    //         return Err(anyhow!("command 'w' did not produce a valid output"));
    //     }

    //     sessions.push(Session {
    //         user: parts[0].to_owned(),
    //         from: parts[2].to_owned(),
    //         login: parts[3].to_owned(),
    //         what: parts[7..].join(" "),
    //     });
    // }

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

    let wg_peers = Vec::<WgPeer>::new();

    // let lines = str::from_utf8(&output.stdout)?.lines();
    // while let Some(line) = lines.next() {
    //     let section = line.split_whitespace().collect::<Vec<_>>();
    //     match section.as_slice() {
    //         // handles interface and all its peers
    //         ["interface:", interface_ref, ..] => {
    //             let interface = (*interface_ref).to_owned();

    //             lines.next();
    //         },
    //         _ => continue // don't care
    //     }
    // }

    Ok(wg_peers)
}

/// Returns a tuple of user sessions and wireguard peers.
pub fn get_report() -> Result<(Vec<Session>, Vec<WgPeer>)> {
    let sessions = get_sessions()?;
    let wg_peers = get_wg_peers()?;

    Ok((sessions, wg_peers))
}
