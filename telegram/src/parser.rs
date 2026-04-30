use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use secmon::models::hub::Node;
use secmon::models::node::{AuthLog, AuthLogDetail, NodeDataError, NodeUpdate};
use secmon::models::packet::ResultStatus;

/// Returns node update parsed in user-friendly format.
///
/// `None` result means that the update is not worth notifying the user.
pub fn parse_node_update(node: &Node, data: &NodeUpdate) -> Option<String> {
    let mut result = format!("<b>{}</b> (<code>{}</code>)\n", node.hostname, node.address);
    let mut ok = false;

    macro_rules! get_pid {
        ( $process:expr ) => {
            $process[$process.find("[").unwrap() + 1..$process.find("]").unwrap() - 1].to_owned()
        };
    }

    if let Some(ref auth_log_res) = data.auth_log {
        ok = true;
        match auth_log_res {
            Ok(AuthLog {
                time: _,
                process,
                user,
                detail,
            }) => match detail {
                AuthLogDetail::SshConnect((host, port), method) => {
                    result += &format!(
                        "[SSH] <code>{user}</code> connected from <code>{host}:{port}</code> using {method}.\n"
                    )
                }
                AuthLogDetail::SshFailPassword((host, port)) => {
                    result += &format!(
                        "[SSH] <b>FAILED</b> password login attempt for <code>{user}</code> from <code>{host}:{port}</code>.\n"
                    )
                }
                AuthLogDetail::SshDisconnect((host, port)) => {
                    result += &format!(
                        "[SSH] <code>{user}</code> disconnected from <code>{host}:{port}</code>.\n"
                    )
                }
                AuthLogDetail::SuOpen(target) => {
                    result += &format!(
                        "[SU] <code>{user}</code> opened session for <code>{target}</code> (pid=<code>{}</code>).\n",
                        get_pid!(process)
                    )
                }
                AuthLogDetail::SuFail(target) => {
                    result += &format!(
                        "[SU] <code>{user}</code> FAILED to open session for <code>{target}</code> (pid=<code>{}</code>).\n",
                        get_pid!(process)
                    )
                }
                AuthLogDetail::SuClose(target) => {
                    result += &format!(
                        "[SU] A session for <code>{target}</code> was closed (pid=<code>{}</code>).\n",
                        get_pid!(process)
                    )
                }
            },
            Err(e) => result += &format!("[ERROR] {e}"),
        }
    }

    if ok { Some(result) } else { None }
}

/// Returns single node parsed in user-friendly format.
pub fn parse_node(tz: &Tz, node: &Node) -> String {
    let mut ret = String::new();

    let connected = match node.connected {
        true => "✅",
        false => "❌",
    };
    ret += format!(
        "{connected} <b>{}</b> (<code>{}</code>)\n",
        node.hostname, node.address
    )
    .as_str();

    macro_rules! format_err {
        ( $err:expr, $attr:expr ) => {
            match $err {
                NodeDataError::Initializing => {
                    format!("{}: Initializing\n", $attr)
                }
                NodeDataError::NotMonitored => {
                    format!("")
                }
                NodeDataError::Message(msg) => {
                    format!("{}: {}\n", $attr, msg)
                }
            }
        };
    }

    match &node.sessions {
        Ok(sessions) => {
            if sessions.len() > 0 {
                ret += "sessions:\n";
                let max_user_len = sessions
                    .into_iter()
                    .map(|session| session.user.len())
                    .max()
                    .unwrap_or(0);
                sessions.into_iter().for_each(|session| {
                        let dt: DateTime<Utc> = session.login.into();
                        let dt: DateTime<Tz> = dt.with_timezone(tz);
                        let from = if let Some(from) = &session.from {
                            format!("({from})")
                        } else {
                            format!("(/)")
                        };
                        ret += format!(
                            "  <code>{user: <user_width$}</code><code>{login: <7}</code><code>{from}</code>\n",
                            user = session.user,
                            user_width = max_user_len + 2,
                            login = dt.format("%H:%M"),
                            from = from
                        )
                        .as_str();
                    });
            }
        }
        Err(e) => ret += format_err!(e, "sessions").as_str(),
    }

    match &node.wg_peers {
        Ok(wg_peers) => {
            if wg_peers.len() > 0 {
                ret += "wg peers:\n";
                wg_peers.into_iter().for_each(|wg_peer| {
                    ret += format!("  <code>{}</code>", wg_peer.interface).as_str();
                    if let Some(endpoint) = &wg_peer.endpoint {
                        ret += format!("  <code>{}</code>", endpoint).as_str();
                    }
                    if let Some(latest_handshake) = &wg_peer.latest_handshake {
                        let dt: DateTime<Utc> = (*latest_handshake).into();
                        let dt: DateTime<Tz> = dt.with_timezone(tz);
                        let parsed = format!("{}", dt.format("%F %T"));

                        ret += format!("  <code>{}</code>", parsed).as_str();
                    }
                    ret += "\n";
                });
            }
        }
        Err(e) => ret += format_err!(e, "wg peers").as_str(),
    }

    ret
}

/// Returns node list parsed in user-friendly format.
///
/// `None` result means that there is no node in the list.
pub fn parse_node_list(tz: &Tz, nodes: &Vec<Node>) -> Option<String> {
    let mut ret = String::new();

    for node in nodes {
        ret += parse_node(tz, node).as_str();

        if ret != "" {
            ret += "\n";
        }
    }

    ret = ret.as_str().trim().to_owned();
    if ret == String::new() {
        None
    } else {
        Some(ret)
    }
}

/// Returns exec result in user-friendly format.
pub fn parse_result(node: &Node, (status, output): &(ResultStatus, String)) -> String {
    let mut ret = format!("<b>{}</b> (<code>{}</code>)\n", node.hostname, node.address);

    let output = match output.as_str().trim() {
        "" => "<i>No output</i>".to_owned(),
        _ => format!("\n<code>{output}</code>"),
    };

    match status {
        ResultStatus::Timeout => {
            ret += format!("⏰ Timeout: {}", output).as_str();
        }
        ResultStatus::Success => {
            ret += format!("✅ Success: {}", output).as_str();
        }
        ResultStatus::Failure => {
            ret += format!("❌ Failure: {}", output).as_str();
        }
        _ => {
            println!("Received invalid partial result: {output}")
        }
    }

    ret
}
