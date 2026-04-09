use anyhow::{Result, anyhow};
use std::process::Command;

use crate::models::{Session, WgPeer};

/// Returns a list of user sessions based on `w` command output.
pub fn get_sessions() -> Result<Vec<Session>> {
    let output = Command::new("w").output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "in get_sessions, command 'w' did not succeed".to_owned()
        ));
    }

    let sessions = Vec::<Session>::new();

    // TODO: parse sessions

    Ok(sessions)
}

/// Returns a list of wireguard peers based on `wg` command output.
pub fn get_wg_peers() -> Result<Vec<WgPeer>> {
    let output = Command::new("wg").output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "in get_wg_peers, command 'wg' did not succeed".to_owned()
        ));
    }

    let wg_peers = Vec::<WgPeer>::new();

    // TODO: parse wireguard peers

    Ok(wg_peers)
}

/// Returns a tuple of user sessions and wireguard peers.
pub fn get_report() -> Result<(Vec<Session>, Vec<WgPeer>)> {
    let sessions = get_sessions()?;
    let wg_peers = get_wg_peers()?;

    Ok((sessions, wg_peers))
}