use secmon::models::hub::Node;
use secmon::models::node::{AuthLog, AuthLogDetail, NodeUpdate};

/// Returns node update parsed in user-friendly format.
///
/// `None` result means that the update is not worth notifying the user.
pub fn parse_node_update(node: &Node, data: &NodeUpdate) -> Option<String> {
    let mut result = format!("<b>{}</b> (<code>{}</code>)\n", node.hostname, node.address);
    let mut ok = false;

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
                        "[SSH] <code>{user}</code> connected from <code>{host}:{port}</code> with {method}.\n"
                    )
                }
                AuthLogDetail::SshFailPassword((host, port)) => {
                    result += &format!(
                        "[SSH] FAILED password login attempt for <code>{user}</code> from <code>{host}:{port}</code>.\n"
                    )
                }
                AuthLogDetail::SshDisconnect((host, port)) => {
                    result += &format!(
                        "[SSH] Disconnected <code>{user}</code> from <code>{host}:{port}</code>.\n"
                    )
                }
                AuthLogDetail::SuOpen(target) => {
                    result += &format!(
                        "[<code>{process}</code>] <code>{user}</code> opened session for <code>{target}</code>.\n"
                    )
                }
                AuthLogDetail::SuFail(target) => {
                    result += &format!(
                        "[<code>{process}</code>] <code>{user}</code> FAILED to open session for <code>{target}</code>.\n"
                    )
                }
                AuthLogDetail::SuClose(target) => {
                    result += &format!(
                        "[<code>{process}</code>] A session for <code>{target}</code> was closed.\n"
                    )
                }
            },
            Err(e) => result += &format!("[ERROR] {e}"),
        }
    }

    if ok { Some(result) } else { None }
}
