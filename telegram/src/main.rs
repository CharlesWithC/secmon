use anyhow::{Result, anyhow};
use frankenstein::TelegramApi;
use frankenstein::client_ureq::Bot;
use frankenstein::methods::SendMessageParams;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;

use secmon::models::hub::{ClientCommand, ClientResponse};
use secmon::traits::iosered::IOSerialized;
use secmon::utils::{get_env_var_strict, get_socket_path};

/// Returns a `Bot` with static lifetime.
fn build_bot() -> &'static Bot {
    let token = get_env_var_strict::<String>("TELEGRAM_BOT_TOKEN", None);
    let bot = Bot::new(token.as_str());
    Box::leak(Box::<Bot>::new(bot))
}

/// Thread to subscribe to hub for node updates, and send update to telegram user.
fn thread_node_update(send_message: impl Fn(String)) -> Result<()> {
    match UnixStream::connect(get_socket_path()) {
        Ok(ref mut stream) => {
            stream.write(&ClientCommand::Subscribe)?;

            loop {
                let resp = stream.read::<ClientResponse>()?;
                match resp {
                    ClientResponse::NodeUpdate(..) => {
                        send_message(format!("{resp}"));
                    }
                    _ => panic!("hub sent a response that is not NodeUpdate"),
                }
            }
        }
        Err(e) => Err(anyhow!(
            "Unable to connect to hub daemon; Is hub daemon running?\n{e}"
        )),
    }
}

fn main() {
    let bot = build_bot();

    // helper function to send message with `user_id` copied inside
    let bot_clone = bot.clone();
    let user_id = get_env_var_strict::<i64>("TELEGRAM_USER_ID", None);
    let send_message = move |text: String| -> () {
        let send_message_params = SendMessageParams::builder()
            .chat_id(user_id)
            .text(text)
            .build();
        let _ = bot_clone.send_message(&send_message_params);
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

    loop {
        // do something (todo)
    }
}
