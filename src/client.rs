use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{
    RpcBlockProductionConfig, RpcGetVoteAccountsConfig, RpcLeaderScheduleConfig,
};
use solana_client::rpc_response::RpcContactInfo;
use solana_sdk::native_token::lamports_to_sol;
use solana_sdk::pubkey::Pubkey;

use crate::settings::Validator;

pub struct Client {
    pub validator: Validator,
    pub client: Option<RpcClient>,
}

impl Client {
    pub fn new(validator: &Validator) -> Self {
        Self {
            validator: validator.to_owned(),
            client: Some(RpcClient::new(validator.to_owned().rpc)),
        }
    }

    pub fn get_version(&self) -> String {
        if let Some(client) = &self.client {
            let pubkey = Pubkey::from_str(&self.validator.identity);
            if let Some(info) = get_contact_info(client, &pubkey.unwrap()) {
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
                    let balance = client.get_balance(&key);
                    match balance {
                        Ok(value) => {
                            return lamports_to_sol(value);
                        }
                        Err(err) => {
                            tracing::error!("{:?}", err.kind);
                        }
                    }
                }
                -1.
            }
            Err(_) => -1.,
        }
    }

    pub fn get_identity_balance(&self) -> f64 {
        self.get_balance(self.validator.identity.as_str())
    }

    pub fn get_vote_balance(&self) -> f64 {
        self.get_balance(self.validator.vote.as_str())
    }

    pub fn is_delinquent(&self) -> Option<bool> {
        if let Some(client) = &self.client {
            let result = client.get_vote_accounts_with_config(RpcGetVoteAccountsConfig {
                vote_pubkey: Some(self.validator.vote.clone()),
                ..Default::default()
            });
            return match result {
                Ok(vote) => Some(!vote.delinquent.is_empty()),
                Err(err) => {
                    tracing::error!("{:?}", err.kind);
                    None
                }
            };
        }
        Some(false)
    }

    pub fn get_credits_and_place(&self) -> (usize, u64) {
        if let Some(client) = &self.client {
            let result = client.get_vote_accounts();
            let vote_accounts = result.unwrap();
            let mut current: Vec<(String, u64)> = vote_accounts
                .current
                .iter()
                .map(|vote_account| {
                    let current_epoch_credits = vote_account.epoch_credits.last().unwrap();
                    let current_credits = current_epoch_credits.1 - current_epoch_credits.2;
                    (vote_account.node_pubkey.clone(), current_credits)
                })
                .collect();
            current.sort_by(|a, b| b.1.cmp(&a.1));
            let position_option_value = current.iter().position(|c| c.0 == self.validator.identity);
            return match position_option_value {
                None => (0, 0),
                Some(value) => {
                    let my_credits = current.get(value).unwrap();
                    (value + 1, my_credits.1)
                }
            };
        }
        (0, 0)
    }

    pub fn get_stake_weighted_skip_rate(&self) -> (f64, f64) {
        if let Some(client) = &self.client {
            let result = client.get_vote_accounts();
            let vote_accounts = result.unwrap();

            let skip_rate: HashMap<_, _> = client
                .get_block_production()
                .ok()
                .map(|result| {
                    result
                        .value
                        .by_identity
                        .into_iter()
                        .map(|(identity, (leader_slots, blocks_produced))| {
                            (
                                identity,
                                100. * (leader_slots.saturating_sub(blocks_produced)) as f64
                                    / leader_slots as f64,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            let current_validators: Vec<(u64, Option<f64>)> = vote_accounts
                .current
                .iter()
                .map(|vote_account| {
                    (
                        vote_account.activated_stake,
                        skip_rate.get(&vote_account.node_pubkey).cloned(),
                    )
                })
                .collect();

            let delinquent_validators: Vec<(u64, Option<f64>)> = vote_accounts
                .delinquent
                .iter()
                .map(|vote_account| {
                    (
                        vote_account.activated_stake,
                        skip_rate.get(&vote_account.node_pubkey).cloned(),
                    )
                })
                .collect();

            let validators: Vec<_> = current_validators
                .into_iter()
                .chain(delinquent_validators.into_iter())
                .collect();

            let total_active_stake: u64 = vote_accounts
                .current
                .iter()
                .chain(vote_accounts.delinquent.iter())
                .map(|vote_account| vote_account.activated_stake)
                .sum();

            let (average_skip_rate, average_stake_weighted_skip_rate) = {
                let mut skip_rate_len = 0;
                let mut skip_rate_sum = 0.;
                let mut skip_rate_weighted_sum = 0.;
                for validator in validators.iter() {
                    if let Some(skip_rate) = validator.1 {
                        skip_rate_sum += skip_rate;
                        skip_rate_len += 1;
                        skip_rate_weighted_sum += skip_rate * validator.0 as f64;
                    }
                }

                if skip_rate_len > 0 && total_active_stake > 0 {
                    (
                        skip_rate_sum / skip_rate_len as f64,
                        skip_rate_weighted_sum / total_active_stake as f64,
                    )
                } else {
                    (100., 100.) // Impossible?
                }
            };
            return (average_skip_rate, average_stake_weighted_skip_rate);
        }
        (100., 100.) // Impossible?
    }

    pub fn get_block_production(&self) -> (usize, usize) {
        if let Some(client) = &self.client {
            let block = client.get_block_production_with_config(RpcBlockProductionConfig {
                identity: Some(self.validator.identity.to_string()),
                ..Default::default()
            });
            return match block {
                Ok(bl) => {
                    let val = bl.value.by_identity.get(self.validator.identity.as_str());
                    if let Some(v) = val {
                        (v.0, v.1)
                    } else {
                        (0, 0)
                    }
                }
                Err(err) => {
                    tracing::error!("{:?}", err.get_transaction_error());
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

    pub fn get_slot_count(&self) -> usize {
        if let Some(client) = &self.client {
            let leader = client.get_leader_schedule_with_config(
                None,
                RpcLeaderScheduleConfig {
                    identity: Some(self.validator.identity.to_string()),
                    ..Default::default()
                },
            );
            let result = match leader.unwrap() {
                None => 0,
                Some(slots) => {
                    if let Some(slots_vec) = slots.get(self.validator.identity.as_str()) {
                        slots_vec.len()
                    } else {
                        0
                    }
                }
            };
            return result;
        }
        0
    }

    pub fn get_epoch_info(&self) -> (String, String, f32) {
        if let Some(client) = &self.client {
            let epoch_info = client.get_epoch_info();
            return match epoch_info {
                Ok(value) => {
                    let epoch_num = value.epoch.to_string();
                    let remaining_slots = value.slots_in_epoch - value.slot_index;
                    let average_time_in_ms = client
                        .get_recent_performance_samples(Some(60))
                        .ok()
                        .and_then(|samples| {
                            let (slots, secs) =
                                samples.iter().fold((0, 0), |(slots, secs), sample| {
                                    (slots + sample.num_slots, secs + sample.sample_period_secs)
                                });
                            (secs as u64).saturating_mul(1000).checked_div(slots)
                        });
                    (
                        epoch_num,
                        humantime::format_duration(
                            Duration::from_secs(remaining_slots * average_time_in_ms.unwrap())
                                / 1000,
                        )
                        .to_string(),
                        remaining_slots as f32 / value.slots_in_epoch as f32,
                    )
                }
                Err(_) => (String::from(""), String::from(""), 0.),
            };
        }
        (String::from(""), String::from(""), 0.)
    }
    /*
    pub fn get_stakes(&self) -> f64 {
        use crate::stake::build_stake_state;

        let mut program_accounts_config = RpcProgramAccountsConfig {
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };
        program_accounts_config.filters = Some(vec![
            // Filter by `StakeState::Stake(_, _)`
            rpc_filter::RpcFilterType::Memcmp(rpc_filter::Memcmp::new_base58_encoded(
                0,
                &[2, 0, 0, 0],
            )),
            // Filter by `Delegation::voter_pubkey`, which begins at byte offset 124
            rpc_filter::RpcFilterType::Memcmp(rpc_filter::Memcmp::new_base58_encoded(
                124,
                Pubkey::from_str(self.node.vote.as_str()).unwrap().as_ref(),
            )),
        ]);
        let all_stake_accounts = rpc_client
            .get_program_accounts_with_config(&stake::program::id(), program_accounts_config)?;
        let stake_history_account = rpc_client.get_account(&stake_history::id())?;
        let clock_account = rpc_client.get_account(&sysvar::clock::id())?;
        let clock: Clock = from_account(&clock_account).ok_or_else(|| {
            CliError::RpcRequestError("Failed to deserialize clock sysvar".to_string())
        })?;

        let mut stake_accounts: Vec<CliKeyedStakeState> = vec![];
        for (stake_pubkey, stake_account) in all_stake_accounts {
            if let Ok(stake_state) = stake_account.state() {
                match stake_state {
                    StakeState::Initialized(_) => {
                        if vote_account_pubkeys.is_none() {
                            stake_accounts.push(CliKeyedStakeState {
                                stake_pubkey: stake_pubkey.to_string(),
                                stake_state: build_stake_state(
                                    stake_account.lamports,
                                    &stake_state,
                                    true,
                                    &stake_history,
                                    &clock,
                                ),
                            });
                        }
                    }
                    StakeState::Stake(_, stake) => {
                        if vote_account_pubkeys.is_none()
                            || vote_account_pubkeys
                            .unwrap()
                            .contains(&stake.delegation.voter_pubkey)
                        {
                            stake_accounts.push(CliKeyedStakeState {
                                stake_pubkey: stake_pubkey.to_string(),
                                stake_state: build_stake_state(
                                    stake_account.lamports,
                                    &stake_state,
                                    true,
                                    &stake_history,
                                    &clock,
                                ),
                            });
                            println!("{},{},{}", stake_pubkey, stake_state, stake_account.lamports);
                        }
                    }
                    _ => {}
                }
            }
        }
       /* for (val) in stake_accounts {
            val
        }*/
        return 0.;
    }
    */
}

fn get_contact_info(rpc_client: &RpcClient, identity: &Pubkey) -> Option<RpcContactInfo> {
    rpc_client
        .get_cluster_nodes()
        .ok()
        .unwrap_or_default()
        .into_iter()
        .find(|node| node.pubkey == identity.to_string())
}
