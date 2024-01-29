use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::sleep;
use ureq::{Error, Response};

use crate::client::Client;
use crate::settings::{NodeCheckSettings, Settings};

mod client;
mod settings;

fn read_setting_from_file() -> Settings {
    let mut path_buf = std::env::current_dir().unwrap();
    path_buf.push("settings.json");
    let json_from_file = std::fs::read_to_string(&path_buf)
        .expect(format!("File not found: {:?}", path_buf.to_str()).as_str());
    let settings: Settings = serde_json::from_str(json_from_file.as_str()).unwrap();
    settings
}

fn main() {
    let settings: Settings = read_setting_from_file();
    println!(
        "period: {}",
        settings.timeouts.balance_check_period.as_secs()
    );
    let nodes_check_list: Arc<RwLock<Vec<NodeCheckSettings>>> =
        Arc::new(RwLock::new(settings.nodes));
    println!("{:?}", nodes_check_list);
    // return;
    let deliquency_period = settings.timeouts.deliquency_check_period;
    let balance_period = settings.timeouts.balance_check_period;
    let delinq_list = nodes_check_list.clone();
    let telegram_settings = settings.telegram.clone();
    let deliquency_thread = thread::spawn(move || loop {
        println!("start loop in thread");
        for validator in delinq_list.read().unwrap().iter() {
            println!("check validator: {}", validator.validator.name);
            let client = Client::new(&validator.validator);
            match client.is_delinquent() {
                None => {
                    println!("Validator {} is healthy", client.validator.name);
                }
                Some(value) => {
                    if value {
                        send_message(
                            format!(
                                "<b>{}</b>\npubkey -> {}\n<b>DELINQUENT!!!</b>!!!",
                                client.validator.name.as_str(),
                                &client.validator.identity[..16]
                            ),
                            telegram_settings.token.as_str(),
                            telegram_settings.alert_chat_id,
                        )
                        .expect("Send alert message error");
                    } else {
                        println!("Validator {} is healthy", client.validator.name);
                    }
                }
            }
        }
        println!("sleep on {:?}", deliquency_period);
        sleep(deliquency_period);
    });
    let balance_check_thread = thread::spawn(move || {
        let mut nodes_map: HashMap<String, (f64, f64)> = HashMap::new();
        loop {
            for validator in nodes_check_list.read().unwrap().iter() {
                let client = Client::new(&validator.validator);
                let identity_balance = client.get_identity_balance();
                let vote_balance = client.get_vote_balance();
                if nodes_map.contains_key(&client.validator.name) {
                    let prev_value = nodes_map.get(&client.validator.name).unwrap();
                    if prev_value.0 + 0.1 < identity_balance {
                        send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Identity balance increased!!! {}:{}:{}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.0, identity_balance, identity_balance - prev_value.0), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
                        println!(
                            "identity: {};{};{}",
                            prev_value.0,
                            identity_balance,
                            identity_balance - prev_value.0
                        );
                    }
                    if prev_value.1 + 0.1 < vote_balance {
                        send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Vote balance increased!!! {}:{}:{}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.1, vote_balance, vote_balance - prev_value.1), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
                        println!(
                            "vote: {};{};{}",
                            prev_value.1,
                            vote_balance,
                            vote_balance - prev_value.1
                        );
                    }
                }
                nodes_map.insert(client.validator.name, (identity_balance, vote_balance));
            }
            println!("sleep balance thread on {:?}", balance_period);
            sleep(balance_period);
        }
    });
    balance_check_thread.join().expect("");
    deliquency_thread.join().expect("");
    return;
    /*
        for node in settings.nodes {
            let mut client = Client {
                validator: node,
                client: None,
            };
            client.initialize();
            /*
            let skip_rate = client.get_skip_rate();
            let cluster_skip_rate = client.get_stake_weighted_skip_rate().1;
            let epoch_info = client.get_epoch_info();
            let blocks = client.get_block_production();
            let slot_count = client.get_slot_count();
            let mut msg: String;
            if skip_rate >= cluster_skip_rate + client.node.critical_excess_of_skip_rate && epoch_info.2 > 0.5 && blocks.0 as f32 / slot_count as f32 > 0.5 {
                msg = format!("<b>{} [{}]</b> 🔴", client.node.name, client.get_version());
                send_message(format!("<b>{}</b>\npubkey -> {}\n<b>CRITICAL_SKIP_RATE => {}!!!</b>!!!", client.node.name.as_str(), &client.node.identity[..16], skip_rate), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
            } else {
                msg = format!("<b>{} [{}]</b> 🟢", client.node.name, client.get_version());
            }
            msg.push_str("\n\n");
            msg.push_str("<code>");
            msg.push_str(format!("{:^16} | {:^16}\n", "identity", "vote").as_str());
            msg.push_str(format!("{:-<35}\n", "").as_str());
            msg.push_str(format!("{:<16} | {:<16}\n", &client.node.identity[..16].to_string(), &client.node.vote[..16].to_string()).as_str());
            let identity_balance = client.get_identity_balance();
            msg.push_str(format!("{:^16.*} | {:^16.*}\n", 2, identity_balance, 2, client.get_vote_balance()).as_str());
            let credits_data = client.get_credits_and_place();
            msg.push_str(format!("{:-<35}\n", "").as_str());
            msg.push_str(format!(" place: {:^8.*} | credits: {:^7.*}\n", 0, credits_data.0, 0, credits_data.1).as_str());
            msg.push_str(format!("{:-<35}\n", "").as_str());
            msg.push_str(format!("{}", " progress | skip | skip% | cluster%\n").as_str());
            msg.push_str(format!("{:-<35}\n", "").as_str());
            let progress = slot_count.to_string() + "/" + blocks.0.to_string().as_str();
            msg.push_str(format!("{:^10}|{:^6}|{:^7.2}|{:^9.2}\n", progress, blocks.0 - blocks.1, skip_rate, cluster_skip_rate).as_str());
            msg.push_str(format!("{:-<35}\n", "").as_str());
            msg.push_str(format!("epoch:{:^4}|{:^25}\n", epoch_info.0, epoch_info.1).as_str());
            msg.push_str(format!("{:-<35}\n", "").as_str());
            msg.push_str("</code>");
            let result = send_message(msg, settings.telegram.token.as_str(), settings.telegram.chat_id);
            match result {
                Ok(_) => { println!("Ok"); }
                Err(e) => { println!("Error: {}", e); }
            }
            match client.is_delinquent() {
                None => {}
                Some(value) => {
                    if value {
                        send_message(format!("<b>{}</b>\npubkey -> {}\n<b>DELINQUENT!!!</b>!!!", client.node.name.as_str(), &client.node.identity[..16]), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
                    }
                }
            }
            if identity_balance < client.node.min_alert_amount {
                send_message(format!("<b>{}</b>\npubkey -> {}\n<b>SMALL AMOUNT => {}!!!</b>!!!", client.node.name.as_str(), &client.node.identity[..16], identity_balance), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
            }
    */

        }*/
}

fn send_message(msg: String, token: &str, chat_id: i64) -> Result<Response, Error> {
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
    resp
}
