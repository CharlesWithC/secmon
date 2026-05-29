use anyhow::Result;
use std::env;
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::Duration;

use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var_strict, get_socket_path};
use secmon_http::utils;

mod models;
mod parser;
use models::{Embed, Webhook};

use crate::models::EmbedField;

const USAGE: &str = "Usage:
  secmon-dc                         launch discord integration
  secmon-dc test                    send a test webhook
  secmon-dc help                    show this help message

Environment:
  DISCORD_WEBHOOK_URL=<url>         mandatory
  DISCORD_MESSAGE_CONTENT=<text>    optional; sets message content in webhook";

fn send_webhook(uri: &String, body: &Webhook) -> () {
    let result = ureq::post(uri).send_json(body);
    match result {
        Ok(_) => {}
        Err(e) => println!("Failed to send webhook: {e}"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "help" {
        println!("{USAGE}");
        process::exit(0);
    }

    let webhook = get_env_var_strict::<String>("DISCORD_WEBHOOK_URL", None);
    let content = get_env_var_strict("DISCORD_MESSAGE_CONTENT", Some(String::from("")));

    if args.len() == 2 && args[1] == "test" {
        let body = Webhook {
            content: content,
            embeds: vec![Embed {
                title: "Test Webhook".to_owned(),
                description: "This is a test webhook.".to_owned(),
                fields: Vec::<EmbedField>::new(),
            }],
        };
        send_webhook(&webhook, &body);
        process::exit(0);
    } else if args.len() != 1 {
        eprintln!("Invalid command; Use 'secmon-dc help' for help");
        process::exit(1);
    }

    loop {
        if let Err(e) = || -> Result<()> {
            match UnixStream::connect(get_socket_path()) {
                Ok(ref mut stream) => {
                    stream.write(&ClientCommand::Subscribe)?;

                    loop {
                        let resp = stream.read::<ClientResponse>()?;
                        match resp {
                            ClientResponse::NodeUpdate { node_serial: serial, data } => {
                                let node = utils::find_node(serial.to_string())?;
                                if let Some(embed) = parser::parse_node_update(&node, &data) {
                                    send_webhook(
                                        &webhook,
                                        &Webhook {
                                            content: content.clone(),
                                            embeds: vec![embed],
                                        },
                                    );
                                }
                            }
                            _ => eprintln!("Hub sent a response that is not NodeUpdate"),
                        }
                    }
                }
                Err(e) => Err(e)?,
            }
        }() {
            eprintln!("Unable to connect to hub daemon: {e}");
        }

        thread::sleep(Duration::from_millis(100));
    }
}
