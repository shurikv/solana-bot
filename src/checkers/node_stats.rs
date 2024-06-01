use crate::client::Client;
use crate::send_message;
use crate::settings::{NodeCheckSettings, Settings};
use chrono::Timelike;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

pub fn run(settings: &Settings) -> JoinHandle<()> {
    let nodes_check_list: Arc<RwLock<Vec<NodeCheckSettings>>> =
        Arc::new(RwLock::new(settings.nodes.clone()));
    let telegram_settings = settings.telegram.clone();
    thread::spawn(move || {
        tracing::info!("Start node stats check thread");
        loop {
            let current_minutes = chrono::Utc::now().minute() as u64;
            let current_seconds = chrono::Utc::now().second() as u64;
            let seconds_to_next_hour = 3600 - (current_minutes * 60 + current_seconds);

            tracing::info!("Sleep node stats thread on {}s", seconds_to_next_hour);
            sleep(Duration::from_secs(seconds_to_next_hour));

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
                        telegram_settings.token.as_str(),
                        telegram_settings.alert_chat_id,
                    )
                    .expect("Send alert message error");
                } else if let Some(delinquent) = client.is_delinquent() {
                    if delinquent {
                        msg = format!(
                            "<b>{} [{}]</b> 🔴",
                            client.validator.name,
                            client.get_version()
                        );
                    } else {
                        msg = format!(
                            "<b>{} [{}]</b> 🟢",
                            client.validator.name,
                            client.get_version()
                        );
                    }
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
                msg.push_str(format!("epoch:{:^4}|{:^25}\n", epoch_info.0, epoch_info.1).as_str());
                msg.push_str(format!("{:-<35}\n", "").as_str());

                let activated_stake = client.activated_stake().unwrap_or_default();
                msg.push_str(format!("Active stake |{:^22.2}", activated_stake).as_str());
                msg.push_str(format!("{:-<35}\n", "").as_str());

                msg.push_str("</code>");

                let result = send_message(
                    msg,
                    telegram_settings.token.as_str(),
                    telegram_settings.chat_id,
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
                        telegram_settings.token.as_str(),
                        telegram_settings.alert_chat_id,
                    )
                    .expect("Send alert message error");
                }
            }
        }
    })
}
