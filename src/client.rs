use std::str::FromStr;
use solana_client::rpc_client::RpcClient;
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
}