use frankenstein::TelegramApi;
use frankenstein::client_ureq::Bot;
use frankenstein::methods::SendMessageParams;
use secmon::utils::get_env_var_strict;

fn main() {
    let token = get_env_var_strict::<String>("TELEGRAM_BOT_TOKEN", None);
    let user_id = get_env_var_strict::<i64>("TELEGRAM_USER_ID", None);

    let bot = Bot::new(token.as_str());

    let send_message_params = SendMessageParams::builder()
        .chat_id(user_id)
        .text("Hello Telegram")
        .build();
    let result = bot.send_message(&send_message_params);
    println!("{:?}", result);
}
