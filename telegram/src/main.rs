use anyhow::{Result, anyhow};
use frankenstein::client_ureq::Bot;
use frankenstein::methods::SendMessageParams;
use frankenstein::{ParseMode, TelegramApi};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use secmon::models::hub::{ClientCommand, ClientResponse, Node};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var_strict, get_socket_path};

mod parser;
mod utils;

type NodesMutex = Arc<Mutex<Vec<Node>>>;

/// Returns a `Bot` with static lifetime.
fn build_bot() -> &'static Bot {
    let token = get_env_var_strict::<String>("TELEGRAM_BOT_TOKEN", None);
    let bot = Bot::new(token.as_str());
    Box::leak(Box::<Bot>::new(bot))
}

/// Thread to subscribe to hub for node updates, and send update to telegram user.
fn thread_node_update(send_message: impl Fn(String), nodes_mutex: &NodesMutex) -> Result<()> {
    match UnixStream::connect(get_socket_path()) {
        Ok(ref mut stream) => {
            stream.write(&ClientCommand::Subscribe)?;

            loop {
                let resp = stream.read::<ClientResponse>()?;
                match resp {
                    ClientResponse::NodeUpdate(serial, data) => {
                        let node = utils::find_node(serial, nodes_mutex, true)?;
                        send_message(parser::parse_node_update(&node, &data));
                    }
                    _ => Err(anyhow!("Hub sent a response that is not NodeUpdate"))?, // should not happen
                }
            }
        }
        Err(e) => Err(anyhow!("Unable to connect to hub daemon: {e}")),
    }
}

fn main() {
    let bot = build_bot();
    let nodes_mutex = Arc::new(Mutex::new(Vec::<Node>::new()));

    // helper function to send message with `user_id` copied inside
    let user_id = get_env_var_strict::<i64>("TELEGRAM_USER_ID", None);
    let send_message = move |text: String| -> () {
        let send_message_params = SendMessageParams::builder()
            .chat_id(user_id)
            .text(text)
            .parse_mode(ParseMode::Html)
            .build();
        let result = bot.send_message(&send_message_params);
        if let Err(e) = result {
            println!("Failed to send message: {e}");
        }
    };

    // thread to handle node updates
    let send_message_clone = send_message.clone();
    let nodes_mutex_clone = Arc::clone(&nodes_mutex);
    thread::spawn(move || {
        // note: this thread does not terminate until main program is terminated
        let mut last_error = String::from("");
        loop {
            let result = thread_node_update(&send_message_clone, &nodes_mutex_clone);
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

    loop {
        // handle user commands (todo)
    }
}
