use anyhow::{Result, anyhow};
use frankenstein::client_ureq::Bot;
use frankenstein::methods::{GetUpdatesParams, SendMessageParams};
use frankenstein::types::AllowedUpdate;
use frankenstein::updates::UpdateContent;
use frankenstein::ureq;
use frankenstein::{ParseMode, TelegramApi};
use std::env;
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::Duration;

use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::models::packet::Command;
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var, get_env_var_strict, get_socket_path, read_lines};

mod parser;
mod utils;

const ARGS: [&str; 3] = ["help", "upd", "exec"];

const USAGE: &str = "Usage:
  secmon-tg [upd] [exec]            launch telegram bot
  secmon-tg help                    show this help message

  Unlike core cli client, remote command execution here does not stream result,
  and so the bot may appear to stall when remotely executing a slow command.

Environment:
  TELEGRAM_BOT_TOKEN=<token>        mandatory
  TELEGRAM_USER_ID=<user-id>        mandatory; user authorized to use the bot
  IPV4_ONLY=<true|false>            optional; use if ipv6 connection times out
  COMMAND_ALLOWLIST_FILE=<path>     optional; necessary for [exec] to function

COMMAND_ALLOWLIST_FILE:
  A file containing commands that may be executed remotely by authorized user.
  This file is supposed to filter allowed commands for security purposes.
  Provide one command label that matches some node command label in each line.
  Examples:
    LABEL
    update
    reboot";

/// Returns a `Bot` with static lifetime.
fn build_bot() -> &'static Bot {
    let mut config_builder = ureq::config::Config::builder()
        .http_status_as_error(false)
        .ip_family(ureq::config::IpFamily::Ipv4Only)
        .timeout_global(Some(Duration::from_secs(500)));
    if get_env_var_strict("IPV4_ONLY", Some(false)) {
        println!("[NOTE] IPV4_ONLY is enabled");
        config_builder = config_builder.ip_family(ureq::config::IpFamily::Ipv4Only)
    }
    let agent = ureq::Agent::new_with_config(config_builder.build());

    let token = get_env_var_strict::<String>("TELEGRAM_BOT_TOKEN", None);

    let bot = Bot::builder()
        .api_url(format!("{}{token}", frankenstein::BASE_API_URL))
        .request_agent(agent)
        .build();

    Box::leak(Box::<Bot>::new(bot))
}

/// Thread to subscribe to hub for node updates, and send update to telegram user.
fn thread_node_update(send_message: &impl Fn(Option<String>)) -> Result<()> {
    match UnixStream::connect(get_socket_path()) {
        Ok(ref mut stream) => {
            stream.write(&ClientCommand::Subscribe)?;

            loop {
                let resp = stream.read::<ClientResponse>()?;
                match resp {
                    ClientResponse::NodeUpdate(serial, data) => {
                        let node = utils::find_node(serial.to_string())?;
                        send_message(parser::parse_node_update(&node, &data));
                    }
                    _ => Err(anyhow!("Hub sent a response that is not NodeUpdate"))?, // should not happen
                }
            }
        }
        Err(e) => Err(anyhow!("Unable to connect to hub daemon: {e}")),
    }
}

/// Handles a message from the telegram user.
fn handle_message(
    enable_exec: bool,
    send_message: &impl Fn(Option<String>),
    message: String,
) -> Result<()> {
    match message.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["list"] | ["-"] => {
            let mut nodes = utils::list_nodes()?;
            nodes.sort_by(|a, b| a.address.cmp(&b.address));
            send_message(parser::parse_node_list(&nodes));
        }
        [node] => {
            let node = utils::find_node((*node).to_owned())?;
            send_message(Some(parser::parse_node(&node)));
        }
        [node, label @ ..] => {
            if !enable_exec {
                send_message(Some(format!("Remote command execution is not enabled")));
                return Ok(());
            }

            let label = label.join(" ");

            match get_env_var::<String>("COMMAND_ALLOWLIST_FILE", None).unwrap() {
                None => {
                    send_message(Some(
                        "Command allowlist not set (Missing env var: COMMAND_ALLOWLIST_FILE)"
                            .to_owned(),
                    ));
                }

                Some(allowlist_file) => match read_lines(allowlist_file.clone()) {
                    Err(e) => {
                        send_message(Some(format!(
                            "Unable to read '{allowlist_file}' (COMMAND_ALLOWLIST_FILE): {:?}",
                            e
                        )));
                    }
                    Ok(lines) => {
                        for line in lines.map_while(Result::ok) {
                            if label == line {
                                let command = Command::Execute(label, false);

                                if node == &"-" {
                                    let nodes = utils::list_nodes()?;

                                    for (i, node) in
                                        nodes.into_iter().filter(|node| node.connected).enumerate()
                                    {
                                        if i != 0 {
                                            println!("");
                                        }

                                        let result = utils::remote_exec(&node, command.clone())?;
                                        send_message(Some(parser::parse_result(&node, &result)));
                                    }
                                } else {
                                    let node = utils::find_node((*node).to_owned())?;
                                    let result = utils::remote_exec(&node, command)?;
                                    send_message(Some(parser::parse_result(&node, &result)));
                                }

                                return Ok(());
                            }
                        }

                        send_message(Some(format!("'{label}' is not an allowed command")));
                    }
                },
            }
        }
        _ => {
            send_message(Some(format!("Unknown command: <code>{message}</code>")));
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    }

    let enable_upd = args.contains(&"upd".to_owned());
    let enable_exec = args.contains(&"exec".to_owned());

    if args[1..].iter().any(|a| !ARGS.contains(&a.as_str())) {
        eprintln!("Invalid command; Use 'secmon-tg help' for help");
        process::exit(1);
    }

    let bot = build_bot();

    // helper function to send message with `user_id` copied inside
    let bot_clone = bot.clone();
    let owner_id = get_env_var_strict::<u64>("TELEGRAM_USER_ID", None);
    let send_message = move |text: Option<String>| -> () {
        match text {
            None => return,
            Some(text) => {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(owner_id as i64)
                    .text(text)
                    .parse_mode(ParseMode::Html)
                    .build();
                let result = bot_clone.send_message(&send_message_params);
                if let Err(e) = result {
                    println!("Failed to send message: {e}");
                }
            }
        }
    };

    // thread to handle node updates
    if enable_upd {
        let send_message = send_message.clone();
        thread::spawn(move || {
            // note: this thread does not terminate until main program is terminated
            let mut last_error = String::from("");
            loop {
                let result = thread_node_update(&send_message);
                if let Err(e) = result {
                    let e_str = format!("{e}");
                    if last_error != e_str {
                        // only log error if not the same error
                        eprintln!("{e_str}");
                        last_error = e_str;
                    }
                }

                // retry every 100ms
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    let mut offset = 0;
    loop {
        let update_params = GetUpdatesParams::builder()
            .allowed_updates(vec![AllowedUpdate::Message])
            .offset(offset as i64 + 1)
            .build();
        let result = bot.get_updates(&update_params);

        match result {
            Ok(resp) => {
                if !resp.ok {
                    eprintln!(
                        "Failed to get updates: {}",
                        resp.description.unwrap_or("Unknown error".to_owned())
                    );
                    thread::sleep(Duration::from_secs(5));
                } else {
                    for update in resp.result {
                        offset = update.update_id;
                        match update.content {
                            UpdateContent::Message(message) => {
                                let (user_id, username) = message
                                    .from
                                    .map(|user| (user.id, user.username.unwrap_or(String::new())))
                                    .unwrap_or((0, String::new()));
                                if user_id != owner_id {
                                    println!("Ignored message from {username} ({user_id})");
                                    continue;
                                }

                                if let Some(message) = message.text {
                                    if let Err(e) =
                                        handle_message(enable_exec, &send_message, message)
                                    {
                                        send_message(Some(format!("Error: {e}")));
                                    }
                                }
                            }
                            _ => {
                                eprintln!(
                                    "Received invalid update content; This should not happen"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get updates: {e}");
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
