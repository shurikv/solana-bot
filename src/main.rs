use crate::client::Client;
use crate::settings::Settings;

mod settings;
mod client;

pub fn read_setting_from_file() -> Settings {
    let json_from_file = std::fs::read_to_string("settings.json").unwrap();
    let settings: Settings = serde_json::from_str(json_from_file.as_str()).unwrap();
    settings
}

fn main() {
    let settings: Settings = read_setting_from_file();
    for node in settings.nodes {
        let mut client = Client {
            node,
            client: None
        };
        client.initialize();

        telegram_notifyrs::send_message(format!("balance: {}", client.get_identity_balance()), settings.telegram.token.as_str(), settings.telegram.chat_id);
    }
}
