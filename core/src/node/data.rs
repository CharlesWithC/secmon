use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use secmon::models::node::{AuthLog, AuthLogDetail, Session, WgPeer};
use secmon::traits::exec::{CommandExec, ExecResult};

/// Executes `who -w` and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_sessions() -> Result<Vec<Session>, String> {
    let mut sessions = Vec::<Session>::new();

    // on ubuntu >= 25.10, `who` gives empty output, but `gnuwho` still gives output
    // thus, we first try `gnuwho` (if it exists, then we're likely on latest ubuntu),
    // and fallback to `who` (which implies we're likely not on latest ubuntu)
    // note: if `gnuwho` fails in the future, we might have to consider /run/systemd/sessions
    //       though it is marked as "This is private data. Do not parse.".
    let res_who: ExecResult;
    let mut command = Command::new("gnuwho");
    command.args(["-w"]).env("LC_ALL", "C.UTF-8");
    if let Ok(result) = command.run(None) {
        // `gnuwho` exists
        res_who = result;
    } else {
        // `gnuwho` does not exist (or forwhatever reason we cannot execute it)
        let mut command = Command::new("who");
        command.args(["-w"]).env("LC_ALL", "C.UTF-8");
        res_who = command.run(None)?;
    }
    if !res_who.status.success() {
        return Err(format!("{}", res_who));
    }

    for line in res_who.output.lines() {
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

/// Executes `wg show all dump` and returns parsed result.
///
/// If an error occurs, returns a string-based error.
pub fn get_wg_peers() -> Result<Vec<WgPeer>, String> {
    let mut wg_peers = Vec::<WgPeer>::new();

    let mut command = Command::new("wg");
    command.args(["show", "all", "dump"]);

    let res_endpoints = command.run(None)?;
    if !res_endpoints.status.success() {
        return Err(format!("{}", res_endpoints));
    }

    for line in res_endpoints.output.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() == 9 {
            wg_peers.push(WgPeer {
                interface: parts[0].to_owned(),
                peer: parts[1].to_owned(),
                endpoint: match parts[3] {
                    "(none)" => None,
                    endpoint @ _ => Some(endpoint.to_owned()),
                },
                latest_handshake: match parts[5].parse::<u64>() {
                    Ok(timestamp) if timestamp != 0 => {
                        Some(UNIX_EPOCH + Duration::from_secs(timestamp))
                    }
                    _ => None,
                },
            });
        }
    }

    Ok(wg_peers)
}

/// Parses a single line of journalctl log and returns the parsed result.
///
/// Note: We assume usernames do not contain spaces.
///       Also, you should NOT allow password login in ssh at all!
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
                detail: AuthLogDetail::SshConnect {
                    host: host.to_owned(),
                    port: port.parse()?,
                    method: method.to_owned(),
                },
            })),
        #[rustfmt::skip]
        ["Failed", "password", "for", user, "from", host, "port", port, ..] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SshFailPassword {
                    host: host.to_owned(),
                    port: port.parse()?,
                },
            })),
        #[rustfmt::skip]
        ["Disconnected", "from", "user", user, host, "port", port] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SshDisconnect {
                    host: host.to_owned(),
                    port: port.parse()?,
                },
            })),
        #[rustfmt::skip]
        ["pam_unix(su:session):", "session", "opened", "for", "user", target_user, "by", user] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.split("(").next().unwrap().to_owned(),
                detail: AuthLogDetail::SuOpen {
                    target_user: target_user.split("(").next().unwrap().to_owned(),
                },
            })),
        #[rustfmt::skip]
        ["FAILED", "SU", "(to", target_user, user, ..] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: user.to_owned(),
                detail: AuthLogDetail::SuFail {
                    target_user: target_user[..target_user.len() - 1].to_owned(),
                },
            })),
        #[rustfmt::skip]
        ["pam_unix(su:session):", "session", "closed", "for", "user", target_user] =>
            Ok(Some(AuthLog {
                time,
                process,
                user: String::from(""),
                detail: AuthLogDetail::SuClose {
                    target_user: target_user.to_owned(),
                },
            })),
        _ => Ok(None), // don't care if no match
    }
}
