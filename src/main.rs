use std::path::Path;
use std::time::Duration;

use serde_json::{json, Map, Value};
use solana_client::rpc_client::RpcClient;
use ureq;
use ureq::{Error, Response};

use crate::client::Client;
use crate::settings::Settings;

mod settings;
mod client;

pub fn read_setting_from_file() -> Settings {
    let mut path_buf = std::env::current_exe().unwrap();
    path_buf.pop();
    path_buf.push("settings.json");
    let json_from_file = std::fs::read_to_string(path_buf.to_str().unwrap()).expect(format!("File not found: {:?}", path_buf.to_str()).as_str());
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

        let mut msg = format!("<b>{} [{}]</b>", client.node.name, client.get_version());
        msg.push_str("\n\n");
        msg.push_str("<code>");
        msg.push_str(format!("{:^16} | {:^16}\n", "identity", "vote").as_str());
        msg.push_str(format!("{:-<35}\n", "").as_str());
        msg.push_str(format!("{:<16} | {:<16}\n", &client.node.identity[..16].to_string(), &client.node.vote[..16].to_string()).as_str());
        msg.push_str(format!("{:^16.*} | {:^16.*}\n", 2, client.get_identity_balance(), 2, client.get_vote_balance()).as_str());
        msg.push_str(format!("{:-<35}\n", "").as_str());
        msg.push_str(format!("{}", " progress | skip | skip% | cluster%\n").as_str());
        msg.push_str(format!("{:-<35}\n", "").as_str());
        let blocks = client.get_block_production();
        let progress = client.get_slot_count().to_string() + "/" + blocks.0.to_string().as_str();
        msg.push_str(format!("{:^10}|{:^6}|{:^7.2}|{:^9.2}\n", progress, blocks.0 - blocks.1, client.get_skip_rate(), client.get_cluster_skip()).as_str());
        msg.push_str(format!("{:-<35}\n", "").as_str());
        let epoch_info = client.get_epoch_info();
        msg.push_str(format!("epoch:{:^4}|{:^25}\n", epoch_info.0, epoch_info.1).as_str());
        msg.push_str(format!("{:-<35}\n", "").as_str());
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