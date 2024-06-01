use crate::client::Client;
use crate::send_message;
use crate::settings::{NodeCheckSettings, Settings};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};

pub fn run(settings: &Settings) -> JoinHandle<()> {
    let nodes_check_list: Arc<RwLock<Vec<NodeCheckSettings>>> =
        Arc::new(RwLock::new(settings.nodes.clone()));
    let balance_period = settings.timeouts.balance_check_period;
    let balance_check_list = nodes_check_list.clone();
    let telegram_settings = settings.telegram.clone();

    thread::spawn(move || {
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
                    if (prev_value.0 - identity_balance).abs() > 0.05 && identity_balance >= 0. {
                        send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Identity balance changed!!! {:.3};{:.3};{:.3}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.0, identity_balance, identity_balance - prev_value.0), telegram_settings.token.as_str(), telegram_settings.alert_chat_id).expect("Send alert message error");
                        tracing::info!(
                            "identity: {:.3};{:.3};{:.3}",
                            prev_value.0,
                            identity_balance,
                            identity_balance - prev_value.0
                        );
                    }
                    if (prev_value.1 - vote_balance).abs() > 0. && vote_balance >= 0. {
                        send_message(format!("<b>{}</b>\npubkey -> {}\n<b>Vote balance changed!!! {:.3};{:.3};{:.3}</b>!!!", client.validator.name.as_str(), &client.validator.identity[..16], prev_value.1, vote_balance, vote_balance - prev_value.1), telegram_settings.token.as_str(), telegram_settings.alert_chat_id).expect("Send alert message error");
                        tracing::info!(
                            "vote: {:.3};{:.3};{:.3}",
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
    })
}
