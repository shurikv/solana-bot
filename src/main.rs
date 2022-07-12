use serde_json::{json, Map, Value};
use ureq;
use ureq::{Error, Response};

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
            client: None,
        };
        client.initialize();

        let mut msg = format!("<b>{}</b>", client.node.name);
        msg.push_str("\n\n");
        msg.push_str("<code>");
        msg.push_str(format!("{:<10} > {}\n", "identity", &client.node.identity[..12].to_string()).as_str());
        msg.push_str(format!("{:<10} > {}\n", "vote", &client.node.vote[..12].to_string()).as_str());
        msg.push_str(format!("{:<10} > {:.*}\n", "balance", 2, client.get_identity_balance()).as_str());
        msg.push_str(format!("{:<10} > {}\n", "version", client.get_version()).as_str());
        msg.push_str("</code>");
        let result = send_message(msg, settings.telegram.token.as_str(), settings.telegram.chat_id);
        match result {
            Ok(_) => { println!("Ok"); }
            Err(e) => { println!("Error: {}", e); }
        }
    }
}

pub fn send_message(
    msg: String,
    token: &str,
    chat_id: i64,
) -> Result<Response, Error> {
    println!("{}", msg);
    let mut request_body = Map::new();
    request_body.insert("text".to_string(), Value::String(msg));
    request_body.insert("chat_id".to_string(), json!(chat_id));
    request_body.insert("parse_mode".to_string(), Value::String("html".to_string()));

    let resp = ureq::post(&format!(
        "https://api.telegram.org/bot{token}/sendMessage",
        token = &token
    ))
        .send_json(json!(request_body));
    return resp;
}