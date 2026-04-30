use anyhow::{Result, anyhow};
use frankenstein::client_ureq::Bot;
use frankenstein::methods::{GetUpdatesParams, SendMessageParams};
use frankenstein::types::AllowedUpdate;
use frankenstein::updates::UpdateContent;
use frankenstein::ureq;
use frankenstein::{ParseMode, TelegramApi};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;

use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var_strict, get_socket_path};

mod parser;
mod utils;

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
fn thread_node_update(send_message: impl Fn(Option<String>)) -> Result<()> {
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
fn handle_message(send_message: impl Fn(Option<String>), message: String) -> Result<()> {
    match message.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["list"] | ["-"] => {
            let nodes = utils::list_nodes()?;
            send_message(parser::parse_node_list(&nodes));
        }
        [node] => {
            let node = utils::find_node((*node).to_owned())?;
            send_message(Some(parser::parse_node(&node)));
        }
        _ => {
            send_message(Some(format!("Unknown command: <code>{message}</code>")));
        }
    }
    Ok(())
}

fn main() {
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
    let send_message_clone = send_message.clone();
    thread::spawn(move || {
        // note: this thread does not terminate until main program is terminated
        let mut last_error = String::from("");
        loop {
            let result = thread_node_update(&send_message_clone);
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
                                    if let Err(e) = handle_message(send_message.clone(), message) {
                                        send_message.clone()(Some(format!("Error: {e}")));
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
