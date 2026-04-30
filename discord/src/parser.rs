use secmon::models::hub::Node;
use secmon::models::node::{AuthLog, AuthLogDetail, NodeUpdate};

use crate::models::{Embed, EmbedField};

/// Returns node update parsed into a Discord embed.
///
/// `None` result means that the update is not worth sending a webhook update.
pub fn parse_node_update(node: &Node, data: &NodeUpdate) -> Option<Embed> {
    let mut embed = Embed::new();
    let mut ok = false;

    embed.description = format!("**{}** (`{}`)", node.hostname, node.address);

    macro_rules! get_pid {
        ( $process:expr ) => {
            format!(
                "`{}`",
                $process[$process.find("[").unwrap() + 1..$process.find("]").unwrap() - 1]
                    .to_owned()
            )
        };
    }

    macro_rules! set_title {
        ( $title:expr ) => {
            embed.title = $title.to_owned();
        };
    }

    macro_rules! add_field {
        ( $name:expr, $value:expr, $inline:expr ) => {
            embed.fields.push(EmbedField {
                name: $name.to_owned(),
                value: $value.to_owned(),
                inline: $inline,
            });
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
                    set_title!("SSH connection opened");
                    add_field!("User", user, true);
                    add_field!("Method", method, true);
                    add_field!("Origin", format!("`{host}:{port}`"), true);
                }
                AuthLogDetail::SshFailPassword((host, port)) => {
                    set_title!(":warning: FAILED SSH PASSWORD LOGIN");
                    add_field!("User", user, true);
                    add_field!("Origin", format!("`{host}:{port}`"), true);
                }
                AuthLogDetail::SshDisconnect((host, port)) => {
                    set_title!("SSH connection closed");
                    add_field!("User", user, true);
                    add_field!("Origin", format!("`{host}:{port}`"), true);
                }
                AuthLogDetail::SuOpen(target) => {
                    set_title!("SU session opened");
                    add_field!("User", user, true);
                    add_field!("Target", target, true);
                    add_field!("PID", get_pid!(process), true);
                }
                AuthLogDetail::SuFail(target) => {
                    set_title!(":warning: FAILED SU SESSION OPEN");
                    add_field!("User", user, true);
                    add_field!("Target", target, true);
                    add_field!("PID", get_pid!(process), true);
                }
                AuthLogDetail::SuClose(target) => {
                    set_title!("SU session closed");
                    add_field!("Target", target, true);
                    add_field!("PID", get_pid!(process), true);
                }
            },
            Err(e) => {
                set_title!("ERROR");
                embed.description += format!("\n\n`{e}").as_str();
            }
        }
    }

    if ok { Some(embed) } else { None }
}
