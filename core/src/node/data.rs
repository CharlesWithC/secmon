use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::node::{AuthLog, AuthLogDetail, Session, WgPeer};
use crate::traits::exec::CommandExec;

/// Executes `who -w` and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_sessions() -> Result<Vec<Session>, String> {
    let mut command = Command::new("who");
    command.args(["-w"]).env("LC_ALL", "C.UTF-8");
    let output = command.run(None)?.output;

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

        // TIME (C.UTF-8 / YYYY-MM-DD HH:MM)
        let login_str = format!("{} {}", parts[len - roffset - 1], parts[len - roffset]);
        let login_dt = NaiveDateTime::parse_from_str(login_str.as_str(), "%Y-%m-%d %H:%M")
            .map_err(|e| format!("unable to parse login time: {e}"))?
            .and_local_timezone(Local)
            .unwrap();
        let login: SystemTime = login_dt.into();

        sessions.push(Session { user, from, login });
    }

    Ok(sessions)
}

/// Executes `wg` twice and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_wg_peers() -> Result<Vec<WgPeer>, String> {
    // `wg show` only supports one attribute in each run
    // the output format is (interface, peer, <attribute>)
    // thus, we run `wg show` twice with different arguments

    let mut wg_peers = Vec::<WgPeer>::new();

    let mut command = Command::new("wg");
    command.args(["show", "all", "endpoints"]);
    let output_endpoints = command.run(None)?.output;
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

    let mut command = Command::new("wg");
    command.args(["show", "all", "latest-handshakes"]);
    let output_handshake = command.run(None)?.output;
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

/// Parses a single line of journalctl log and returns the parsed result.
pub fn parse_journalctl_log(line: String) -> Result<Option<AuthLog>> {
    let parts = line.split_whitespace().collect::<Vec<_>>();

    let dt = DateTime::parse_from_str(parts[0], "%Y-%m-%dT%H:%M:%S%z")?;
    let time: SystemTime = dt.into();

    let process = parts[2][..parts[2].len() - 1].to_owned();

    match parts.as_slice()[3..] {
        #[rustfmt::skip]
        ["Accepted", method, "for", user, "from", host, "port", port, ..] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SshConnect((host.to_owned(), port.parse()?), method.to_owned()),
            })),
        #[rustfmt::skip]
        ["Failed", "password", "for", user, "from", host, "port", port, ..] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SshFailPassword((host.to_owned(), port.parse()?)),
            })),
        #[rustfmt::skip]
        ["Disconnected", "from", "user", user, host, "port", port] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SshDisconnect((host.to_owned(), port.parse()?)),
            })),
        #[rustfmt::skip]
        ["pam_unix(su:session):", "session", "opened", "for", "user", target_user, "by", user] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.split("(").next().unwrap().to_owned(),
                detail: AuthLogDetail::SuOpen(target_user.split("(").next().unwrap().to_owned()),
            })),
        #[rustfmt::skip]
        ["FAILED", "SU", "(to", target_user, user, ..] => {
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SuFail(target_user[..target_user.len()-1].to_owned()),
            }))
        },
        #[rustfmt::skip]
        ["pam_unix(su:session):", "session", "closed", "for", "user", target_user] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: String::from(""),
                detail: AuthLogDetail::SuClose(target_user.to_owned()),
            })),
        _ => Ok(None), // don't care if no match
    }
}
