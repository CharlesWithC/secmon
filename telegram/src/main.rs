use anyhow::{Result, anyhow};
use chrono_tz::Tz;
use frankenstein::client_ureq::Bot;
use frankenstein::input_file::InputFile;
use frankenstein::methods::{GetUpdatesParams, SendDocumentParams, SendMessageParams};
use frankenstein::types::AllowedUpdate;
use frankenstein::updates::UpdateContent;
use frankenstein::ureq;
use frankenstein::{ParseMode, TelegramApi};
use std::env;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process;
use std::thread;
use std::time::Duration;

use secmon::GENERIC_OPTIONS;
use secmon::config::get_config;
use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::models::packet::Request;
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_socket_path, init_from_generic_args, sanitize_generic_args};
use secmon_http::utils;

mod config;
mod parser;
use crate::config::{CONFIG, TelegramConfig};

const ARGS: [&str; 2] = ["upd", "exec"];

const USAGE: &str = const_format::formatcp!(
    "Usage:
  secmon-tg [upd] [exec]            launch telegram bot
  secmon-tg help                    show this help message

  Unlike core cli client, remote command execution here does not stream result,
  and so the bot may appear to stall when remotely executing a slow command.

  Options:
    {GENERIC_OPTIONS}"
);

/// Returns a `Bot` with static lifetime.
fn build_bot() -> &'static Bot {
    let mut config_builder = ureq::config::Config::builder()
        .http_status_as_error(false)
        .ip_family(ureq::config::IpFamily::Ipv4Only)
        .timeout_global(Some(Duration::from_secs(500)));
    if get_config!(telegram).ipv4_only {
        println!("[NOTE] IPV4ONLY is enabled");
        config_builder = config_builder.ip_family(ureq::config::IpFamily::Ipv4Only)
    }
    let agent = ureq::Agent::new_with_config(config_builder.build());

    let token = get_config!(telegram).token.clone();

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
                    ClientResponse::NodeUpdate {
                        node_serial: serial,
                        data,
                    } => {
                        let nodes = utils::find_nodes(serial.to_string())?;
                        let node = nodes
                            .iter()
                            .find(|node| node.serial == serial)
                            .ok_or_else(|| anyhow!("Unable to identify node by serial {serial}"))?;
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
    tz: &Tz,
    send_message: &impl Fn(Option<String>),
    send_document: &impl Fn(Option<String>, PathBuf),
    message: String,
) -> Result<()> {
    match message.split_whitespace().collect::<Vec<_>>().as_slice() {
        // note: label may be empty; that indicates listing nodes
        [query @ ("list" | "-" | _), label @ ..] => {
            let mut nodes = match *query {
                "list" | "-" => utils::list_nodes()?,
                query @ _ => utils::find_nodes((*query).to_owned())?,
            };
            secmon::utils::sort_nodes(&mut nodes, get_config!(client));

            if label.is_empty() {
                send_message(parser::parse_node_list(tz, &nodes));
            } else {
                if !enable_exec {
                    send_message(Some(format!("Remote command execution is not enabled")));
                    return Ok(());
                }

                let command_label = label.join(" ");
                if !get_config!(telegram)
                    .command_allowlist
                    .contains(&command_label)
                {
                    send_message(Some(format!("'{command_label}' is not an allowed command")));
                    return Ok(());
                }

                let command = Request::Execute {
                    command_label,
                    stream: false,
                };
                for node in nodes.into_iter().filter(|node| node.connected) {
                    let result = utils::raw_command(&node, command.clone(), 0)?;

                    // note: `_tempdir` is dropped automatically when it's out of scope
                    let (content, _tempdir, file) = parser::parse_result(&node, &result)?;
                    match file {
                        None => send_message(Some(content)),
                        Some(file) => send_document(Some(content), file.into()),
                    }
                }
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
    let sanitized_args = sanitize_generic_args(&args);
    if sanitized_args.len() == 2 && sanitized_args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    }

    init_from_generic_args(&args, &CONFIG).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let enable_upd = sanitized_args.contains(&"upd".to_owned());
    let enable_exec = sanitized_args.contains(&"exec".to_owned());
    sanitized_args[1..].iter().for_each(|a| {
        if !ARGS.contains(&a.as_str()) {
            eprintln!("Unrecognized option: {}\n\n{USAGE}", a);
            process::exit(1);
        }
    });

    let TelegramConfig {
        timezone: tz,
        user_id: owner_id,
        ..
    } = get_config!(telegram);
    let bot = build_bot();

    // helper function to send message with `user_id` copied inside
    // note: if text is `None`, then return directly
    let bot_clone = bot.clone();
    let send_message = move |text: Option<String>| -> () {
        match text {
            None => return,
            Some(text) => {
                let params = SendMessageParams::builder()
                    .chat_id(*owner_id as i64)
                    .text(text)
                    .parse_mode(ParseMode::Html)
                    .build();
                let result = bot_clone.send_message(&params);
                if let Err(e) = result {
                    println!("Failed to send message: {e}");
                }
            }
        }
    };
    // helper function to send file with `user_id` copied inside
    // note: text refers to caption in message
    let bot_clone = bot.clone();
    let send_document = move |text: Option<String>, path: PathBuf| -> () {
        let params = SendDocumentParams::builder()
            .chat_id(*owner_id as i64)
            .maybe_caption(text)
            .parse_mode(ParseMode::Html)
            .document(InputFile::from(path))
            .build();
        let result = bot_clone.send_document(&params);
        if let Err(e) = result {
            println!("Failed to send document: {e}");
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
                                if user_id != *owner_id {
                                    println!("Ignored message from {username} ({user_id})");
                                    continue;
                                }

                                if let Some(message) = message.text {
                                    if let Err(e) = handle_message(
                                        enable_exec,
                                        &tz,
                                        &send_message,
                                        &send_document,
                                        message,
                                    ) {
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
