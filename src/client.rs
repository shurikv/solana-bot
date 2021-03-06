use std::str::FromStr;
use std::time::Duration;

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcBlockProductionConfig, RpcGetVoteAccountsConfig, RpcLeaderScheduleConfig};
use solana_client::rpc_response::{RpcContactInfo, RpcLeaderSchedule};
use solana_sdk::native_token::lamports_to_sol;
use solana_sdk::pubkey::Pubkey;

use crate::settings::Node;

pub struct Client {
    pub node: Node,
    pub client: Option<RpcClient>,
}

impl Client {
    pub fn initialize(&mut self) {
        self.client = Some(RpcClient::new(&self.node.rpc));
    }

    pub fn get_version(&self) -> String {
        if let Some(client) = &self.client {
            let pubkey = Pubkey::from_str(&self.node.identity);
            if let Some(info) = get_contact_info(&client, &pubkey.unwrap()) {
                return info.version.unwrap_or_else(|| "?".to_string());
            }
        }
        "?".to_string()
    }

    fn get_balance(&self, key: &str) -> f64 {
        let pubkey = Pubkey::from_str(key);
        match pubkey {
            Ok(key) => {
                if let Some(client) = &self.client {
                    let balance = client.get_balance(&key).unwrap();
                    return lamports_to_sol(balance);
                }
                -1.
            }
            Err(_) => -1.
        }
    }

    pub fn get_identity_balance(&self) -> f64 {
        self.get_balance(self.node.identity.as_str())
    }

    pub fn get_vote_balance(&self) -> f64 {
        self.get_balance(self.node.vote.as_str())
    }

    pub fn is_delinquent(&self) -> Option<bool> {
        if let Some(client) = &self.client {
            let result = client.get_vote_accounts_with_config(RpcGetVoteAccountsConfig {
                vote_pubkey: Some(self.node.vote.clone()),
                ..Default::default()
            });
            return match result {
                Ok(vote) => {
                    Some(!vote.delinquent.is_empty())
                }
                Err(_) => {
                    None
                }
            };
        }
        Some(false)
    }

    pub fn get_block_production(&self) -> (usize, usize) {
        if let Some(client) = &self.client {
            let block = client.get_block_production_with_config(RpcBlockProductionConfig {
                identity: Some(self.node.identity.to_string()),
                ..Default::default()
            });
            return match block {
                Ok(bl) => {
                    let val = bl.value.by_identity.get(self.node.identity.as_str()).unwrap();
                    (val.0, val.1)
                }
                Err(_) => {
                    (0, 0)
                }
            };
        }
        (0, 0)
    }

    pub fn get_skip_rate(&self) -> f64 {
        let val = self.get_block_production();
        (val.0 - val.1) as f64 * 100. / val.0 as f64
    }

    pub fn get_cluster_skip(&self) -> f64 {
        if let Some(client) = &self.client {
            let result = client.get_block_production();
            return match result {
                Ok(skip) => {
                    skip.value.by_identity.values().fold(0., |x, y| x + (y.0 - y.1) as f64 * 100. / y.0 as f64) / skip.value.by_identity.len() as f64
                }
                Err(_) => {
                    0.
                }
            };
        }
        0.
    }

    pub fn get_slot_count(&self) -> usize {
        if let Some(client) = &self.client {
            let leader = client.get_leader_schedule_with_config(None, RpcLeaderScheduleConfig {
                identity: Some(self.node.identity.to_string()),
                ..Default::default()
            });
            let result = match leader.unwrap() {
                None => { 0 }
                Some(slots) => { slots.get(self.node.identity.as_str()).unwrap().len() }
            };
            return result;
        }
        0
    }

    pub fn get_epoch_info(&self) -> (String, String) {
        if let Some(client) = &self.client {
            let epoch_info = client.get_epoch_info();
            return match epoch_info {
                Ok(value) => {
                    let epoch_num = value.epoch.to_string();
                    let remaining_slots = value.slots_in_epoch - value.slot_index;
                    let average_time_in_ms = client.get_recent_performance_samples(Some(60)).ok()
                        .and_then(|samples| {
                            let (slots, secs) = samples.iter().fold((0, 0), |(slots, secs), sample| {
                                (slots + sample.num_slots, secs + sample.sample_period_secs)
                            });
                            (secs as u64).saturating_mul(1000).checked_div(slots)
                        });
                    (epoch_num, humantime::format_duration(Duration::from_secs(remaining_slots * average_time_in_ms.unwrap()) / 1000).to_string())
                }
                Err(_) => {
                    (String::from(""), String::from(""))
                }
            };
        }
        (String::from(""), String::from(""))
    }
}

fn get_contact_info(rpc_client: &RpcClient, identity: &Pubkey) -> Option<RpcContactInfo> {
    rpc_client
        .get_cluster_nodes()
        .ok()
        .unwrap_or_default()
        .into_iter()
        .find(|node| node.pubkey == identity.to_string())
}

