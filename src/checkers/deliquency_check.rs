use crate::client::Client;
use crate::send_message;
use crate::settings::{NodeCheckSettings, Settings};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};

pub fn run(settings: &Settings) -> JoinHandle<()> {
    let nodes_check_list: Arc<RwLock<Vec<NodeCheckSettings>>> =
        Arc::new(RwLock::new(settings.nodes.clone()));
    let delinq_list = nodes_check_list.clone();
    let delinquency_period = settings.timeouts.deliquency_check_period;
    let telegram_settings = settings.telegram.clone();

    thread::spawn(move || {
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
                            tracing::error!("Validator {} is delinquent", client.validator.name);
                        } else {
                            tracing::trace!("Validator {} is healthy", client.validator.name);
                        }
                    }
                }
            }
            tracing::trace!("Sleep delinquency thread on {:?}", delinquency_period);
            sleep(delinquency_period);
        }
    })
}
