use serde_json::{json, Map, Value};
use ureq::{Error, Response};
use crate::checkers::{balance_check, deliquency_check, node_stats};

use crate::settings::Settings;
use crate::SolanaBotError::ParseSettingsError;

mod checkers;
mod client;
mod logger;
mod settings;

#[derive(Debug)]
pub enum SolanaBotError {
    ParseSettingsError(serde_json::Error),
}

impl From<serde_json::Error> for SolanaBotError {
    fn from(value: serde_json::Error) -> Self {
        ParseSettingsError(value)
    }
}

fn read_setting_from_file() -> Result<Settings, SolanaBotError> {
    let mut path_buf = std::env::current_exe().unwrap();
    path_buf.pop();
    path_buf.push("settings.json");
    let json_from_file = std::fs::read_to_string(&path_buf)
        .unwrap_or_else(|_| panic!("File not found: {:?}", path_buf.to_str()));
    return match serde_json::from_str(json_from_file.as_str()) {
        Ok(value) => Ok(value),
        Err(e) => Err(ParseSettingsError(e)),
    };
}

fn main() {
    logger::setup_logger();
    if let Ok(settings) = read_setting_from_file() {
        let delinquency_thread = deliquency_check::run(&settings);
        let balance_check_thread = balance_check::run(&settings);
        let node_stats_check_thread = node_stats::run(&settings);

        node_stats_check_thread.join().expect("");
        delinquency_thread.join().expect("");
        balance_check_thread.join().expect("");
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
