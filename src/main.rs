use chrono::Timelike;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::sleep;
use std::time::{Duration};
use ureq::{Error, Response};

use crate::client::Client;
use crate::settings::{NodeCheckSettings, Settings};
use crate::SolanaBotError::ParseSettingsError;

mod client;
mod deliquency_check;
mod logger;
mod settings;

pub enum SolanaBotError {
    ParseSettingsError(serde_json::Error),
}

impl From<serde_json::Error> for SolanaBotError {
    fn from(value: serde_json::Error) -> Self {
        ParseSettingsError(value)
    }
}

fn read_setting_from_file() -> Result<Settings, SolanaBotError> {
    let mut path_buf = std::env::current_dir().unwrap();
    path_buf.push("settings.json");
    let json_from_file = std::fs::read_to_string(&path_buf)
        .expect(format!("File not found: {:?}", path_buf.to_str()).as_str());
    return match serde_json::from_str(json_from_file.as_str()) {
        Ok(value) => Ok(value),
        Err(e) => Err(ParseSettingsError(e)),
    };
}

fn main() {
    logger::setup_logger();
    if let Ok(settings) = read_setting_from_file() {
        let nodes_check_list: Arc<RwLock<Vec<NodeCheckSettings>>> =
            Arc::new(RwLock::new(settings.nodes));
        let delinquency_period = settings.timeouts.deliquency_check_period;
        let balance_period = settings.timeouts.balance_check_period;
        let delinq_list = nodes_check_list.clone();
        let balance_check_list = nodes_check_list.clone();
        let telegram_settings = settings.telegram.clone();
        let telegram_settings2 = settings.telegram.clone();
        let delinquency_thread = thread::spawn(move || {
            tracing::info!("Start delinquency thread");
            loop {
                for validator in delinq_list.read().unwrap().iter() {
                    tracing::trace!("Check delinquent for {}", validator.validator.name);
                    let client = Client::new(&validator.validator);
                    match client.is_delinquent() {
                        None => {
                            tracing::trace!("Validator {} is healthy", client.validator.name);
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
                                tracing::error!(
                                    "Validator {} is delinquent",
                                    client.validator.name
                                );
                            } else {
                                tracing::trace!("Validator {} is healthy", client.validator.name);
                            }
                        }
                    }
                }
                tracing::trace!("Sleep delinquency thread on {:?}", delinquency_period);
                sleep(delinquency_period);
            }
        });

        let balance_check_thread = thread::spawn(move || {
            tracing::info!("Start balance check thread");
            let mut nodes_map: HashMap<String, (f64, f64)> = HashMap::new();
            loop {
                for validator in balance_check_list.read().unwrap().iter() {
                    tracing::trace!("Check balance for {}", validator.validator.name);
                    let client = Client::new(&validator.validator);
                    let identity_balance = client.get_identity_balance();
                    let vote_balance = client.get_vote_balance();
                    if nodes_map.contains_key(&client.validator.name) {
                        let prev_value = nodes_map.get(&client.validator.name).unwrap();
                        if (prev_value.0 - identity_balance).abs() > 0.05 && identity_balance >= 0.
                        {
                            send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Identity balance increased!!! {}:{}:{}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.0, identity_balance, identity_balance - prev_value.0), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
                            tracing::info!(
                                "identity: {};{};{}",
                                prev_value.0,
                                identity_balance,
                                identity_balance - prev_value.0
                            );
                        }
                        if (prev_value.1 - vote_balance).abs() > 0.05 && vote_balance >= 0. {
                            send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Vote balance increased!!! {}:{}:{}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.1, vote_balance, vote_balance - prev_value.1), settings.telegram.token.as_str(), settings.telegram.alert_chat_id).expect("Send alert message error");
                            tracing::info!(
                                "vote: {};{};{}",
                                prev_value.1,
                                vote_balance,
                                vote_balance - prev_value.1
                            );
                        }
                    }
                    if identity_balance >= 0. && vote_balance >= 0. {
                        nodes_map.insert(client.validator.name, (identity_balance, vote_balance));
                    }
                }
                tracing::trace!("Sleep balance thread on {:?}", balance_period);
                sleep(balance_period);
            }
        });

        adjust_time();

        let node_stats_check_thread = thread::spawn(move || {
            tracing::info!("Start node stats check thread");
            loop {
                for node in nodes_check_list.read().unwrap().iter() {
                    let client = Client::new(&node.validator);
                    let skip_rate = client.get_skip_rate();
                    let cluster_skip_rate = client.get_stake_weighted_skip_rate().1;
                    let epoch_info = client.get_epoch_info();
                    let blocks = client.get_block_production();
                    let slot_count = client.get_slot_count();
                    let mut msg: String;
                    if skip_rate >= cluster_skip_rate + node.critical_excess_of_skip_rate
                        && epoch_info.2 > 0.5
                        && blocks.0 as f32 / slot_count as f32 > 0.5
                    {
                        msg = format!(
                            "<b>{} [{}]</b> 🔴",
                            client.validator.name,
                            client.get_version()
                        );
                        send_message(
                            format!(
                                "<b>{}</b>\npubkey -> {}\n<b>CRITICAL_SKIP_RATE => {}!!!</b>!!!",
                                client.validator.name.as_str(),
                                &client.validator.identity[..16],
                                skip_rate
                            ),
                            telegram_settings2.token.as_str(),
                            telegram_settings2.alert_chat_id,
                        )
                        .expect("Send alert message error");
                    } else {
                        msg = format!(
                            "<b>{} [{}]</b> 🟢",
                            client.validator.name,
                            client.get_version()
                        );
                    }
                    msg.push_str("\n\n");
                    msg.push_str("<code>");
                    msg.push_str(format!("{:^16} | {:^16}\n", "identity", "vote").as_str());
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    msg.push_str(
                        format!(
                            "{:<16} | {:<16}\n",
                            &client.validator.identity[..16].to_string(),
                            &client.validator.vote[..16].to_string()
                        )
                        .as_str(),
                    );
                    let identity_balance = client.get_identity_balance();
                    msg.push_str(
                        format!(
                            "{:^16.*} | {:^16.*}\n",
                            2,
                            identity_balance,
                            2,
                            client.get_vote_balance()
                        )
                        .as_str(),
                    );
                    let credits_data = client.get_credits_and_place();
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    msg.push_str(
                        format!(
                            " place: {:^8.*} | credits: {:^7.*}\n",
                            0, credits_data.0, 0, credits_data.1
                        )
                        .as_str(),
                    );
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    msg.push_str(" progress | skip | skip% | cluster%\n");
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    let progress = slot_count.to_string() + "/" + blocks.0.to_string().as_str();
                    msg.push_str(
                        format!(
                            "{:^10}|{:^6}|{:^7.2}|{:^9.2}\n",
                            progress,
                            blocks.0 - blocks.1,
                            skip_rate,
                            cluster_skip_rate
                        )
                        .as_str(),
                    );
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    msg.push_str(
                        format!("epoch:{:^4}|{:^25}\n", epoch_info.0, epoch_info.1).as_str(),
                    );
                    msg.push_str(format!("{:-<35}\n", "").as_str());
                    msg.push_str("</code>");
                    let result = send_message(
                        msg,
                        telegram_settings2.token.as_str(),
                        telegram_settings2.chat_id,
                    );
                    match result {
                        Ok(_) => {
                            tracing::info!("Ok");
                        }
                        Err(e) => {
                            tracing::info!("Error: {}", e);
                        }
                    }

                    if identity_balance < node.min_balance_amount {
                        send_message(
                            format!(
                                "<b>{}</b>\npubkey -> {}\n<b>SMALL AMOUNT => {}!!!</b>!!!",
                                client.validator.name.as_str(),
                                &client.validator.identity[..16],
                                identity_balance
                            ),
                            telegram_settings2.token.as_str(),
                            telegram_settings2.alert_chat_id,
                        )
                        .expect("Send alert message error");
                    }
                }
                tracing::trace!("Sleep node stats thread on 1h");
                sleep(Duration::from_secs_f32(3600.));
            }
        });

        node_stats_check_thread.join().expect("");
        delinquency_thread.join().expect("");
        balance_check_thread.join().expect("");
    }
}

fn adjust_time() {
    loop {
        let now = chrono::Utc::now();
        if now.minute() == 0 {
            tracing::info!("Time adjusted");
            break;
        } else {
            sleep(Duration::from_secs(1));
        }
    }
}

fn send_message(msg: String, token: &str, chat_id: i64) -> Result<Response, Error> {
    tracing::info!("{}", msg);
    let mut request_body = Map::new();
    request_body.insert("text".to_string(), Value::String(msg));
    request_body.insert("chat_id".to_string(), json!(chat_id));
    request_body.insert("parse_mode".to_string(), Value::String("html".to_string()));

    ureq::post(&format!(
        "https://api.telegram.org/bot{token}/sendMessage",
        token = &token
    ))
    .send_json(json!(request_body))
}
