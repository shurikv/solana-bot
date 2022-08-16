use serde::{Serialize,Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub telegram: Telegram,
    pub nodes: Vec<Node>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Telegram {
    pub token: String,
    pub chat_id: i64,
    pub alert_chat_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub name: String,
    pub min_alert_amount: f64,
    pub identity: String,
    pub vote: String,
    pub rpc: String,
    pub critical_excess_of_skip_rate: f64,
}
