use anyhow::Result;
use std::env;
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::Duration;

use secmon::GENERIC_OPTIONS;
use secmon::config::get_config;
use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_socket_path, init_from_generic_args, sanitize_generic_args};
use secmon_http::utils;

mod config;
mod models;
mod parser;
use models::{Embed, Webhook};

use crate::config::{CONFIG, DiscordConfig};
use crate::models::EmbedField;

const USAGE: &str = const_format::formatcp!(
    "Usage:
  secmon-dc                         launch discord integration
  secmon-dc test                    send a test webhook
  secmon-dc help                    show this help message

Options:
  {GENERIC_OPTIONS}"
);

fn send_webhook(uri: &String, body: &Webhook) -> () {
    let result = ureq::post(uri).send_json(body);
    match result {
        Ok(_) => {}
        Err(e) => println!("Failed to send webhook: {e}"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let sanitized_args = sanitize_generic_args(&args);
    if sanitized_args.len() == 2 && sanitized_args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    } else if sanitized_args.len() != 1 && sanitized_args[1] != "test" {
        println!("Unrecognized command: {}\n\n{USAGE}", sanitized_args[1]);
        process::exit(1);
    }

    init_from_generic_args(&args, &CONFIG).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    let DiscordConfig {
        webhook,
        message: content,
    } = get_config!(discord);

    if sanitized_args.len() == 2 && sanitized_args[1] == "test" {
        let body = Webhook {
            content: content.clone(),
            embeds: vec![Embed {
                title: "Test Webhook".to_owned(),
                description: "This is a test webhook.".to_owned(),
                fields: Vec::<EmbedField>::new(),
            }],
        };
        send_webhook(&webhook, &body);
        process::exit(0);
    }

    let mut last_error = String::new();
    loop {
        if let Err(e) = || -> Result<()> {
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
                                match nodes.iter().find(|node| node.serial == serial) {
                                    None => eprintln!("Unable to identify node by serial {serial}"),
                                    Some(node) => {
                                        if let Some(embed) = parser::parse_node_update(&node, &data)
                                        {
                                            send_webhook(
                                                &webhook,
                                                &Webhook {
                                                    content: content.clone(),
                                                    embeds: vec![embed],
                                                },
                                            );
                                        }
                                    }
                                };
                            }
                            _ => eprintln!("Hub sent a response that is not NodeUpdate"),
                        }
                    }
                }
                Err(e) => Err(e)?,
            }
        }() {
            let e_str = format!("{e}");
            if last_error != e_str {
                println!("Unable to connect to hub daemon: {e_str}");
                last_error = e_str;
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}
