use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub telegram: Telegram,
    pub timeouts: Timeouts,
    pub nodes: Vec<NodeCheckSettings>,
    pub balances: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Telegram {
    pub token: String,
    pub chat_id: i64,
    pub alert_chat_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeouts {
    #[serde(with = "humantime_serde")]
    pub deliquency_check_period: Duration,
    #[serde(with = "humantime_serde")]
    pub balance_check_period: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts {
            deliquency_check_period: Duration::from_secs(10),
            balance_check_period: Duration::from_secs(5),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCheckSettings {
    pub validator: Validator,
    pub min_balance_amount: f64,
    pub critical_excess_of_skip_rate: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Validator {
    pub name: String,
    pub identity: String,
    pub vote: String,
    pub rpc: String,
}
