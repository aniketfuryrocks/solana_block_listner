use std::time::Instant;

use log::{info, warn};
use serde_json::json;

pub struct ReqwestListner {
    pub rpc_client: reqwest::Client,
    pub rpc_addr: String,
}

impl ReqwestListner {
    pub async fn index_slot(
        &self,
        slot: u64,
        commitment: &str,
        transaction_details: &str,
    ) -> anyhow::Result<()> {
        let block: serde_json::Value = self
            .rpc_client
            .post(&self.rpc_addr)
            .json(&json! ({
                "jsonrpc": "2.0",
                "id":1,
                "method":"getBlock",
                "params": [
                  slot,
                  {
                    "transactionDetails": transaction_details,
                    "commitment": commitment,
                    "maxSupportedTransactionVersion":0,
                    "encoding": "base64",
                    "rewards":false
                  }
                ]
            }))
            .send()
            .await?
            .json()
            .await?;

        let serde_json::Value::Object(block) = &block["result"] else {
            warn!("{}", block["error"]);
            return Ok(());
        };

        let blockhash = &block["blockhash"];
        let parent_slot = &block["parentSlot"];

        let serde_json::Value::Number(block_height) = &block["blockHeight"] else {
            warn!("Received no block height for slot {slot} and blockhash {blockhash}");
            return Ok(());
        };

        let serde_json::Value::Array(transactions) = &block["transactions"] else {
            warn!("No transactions in block");
            return Ok(());
        };

        info!(
            "{slot} at {blockhash} height {block_height} with {} txs and parent {parent_slot}",
            transactions.len()
        );

        Ok(())
    }

    pub async fn listen(self, commitment: &str, transaction_details: &str) -> anyhow::Result<()> {
        let latest_slot: serde_json::Value = self
            .rpc_client
            .post(&self.rpc_addr)
            .json(&json! ({
                "jsonrpc": "2.0",
                "id":1,
                "method":"getSlot",
                "params": [
                  {
                    "commitment": commitment,
                  }
                ]
            }))
            .send()
            .await?
            .json()
            .await?;

        let mut latest_slot = latest_slot["result"].as_u64().unwrap();

        info!(
            "Listening to blocks {commitment:?} with {transaction_details:?} transaction details"
        );

        loop {
            let new_block_slots: serde_json::Value = self
                .rpc_client
                .post(&self.rpc_addr)
                .json(&json! ({
                    "jsonrpc": "2.0",
                    "id":1,
                    "method":"getBlocks",
                    "params": [
                        latest_slot,
                        None::<()>,
                        {
                            "commitment" : commitment
                        }
                    ]
                }))
                .send()
                .await?
                .json()
                .await?;

            let serde_json::Value::Array(new_block_slots) = &new_block_slots["result"] else {
                warn!("{}", new_block_slots["error"]);
                return Ok(());
            };

            if new_block_slots.is_empty() {
                warn!("{latest_slot} No slots");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }

            let Some(new_latest_slot) = new_block_slots.last().cloned() else {
                warn!("Didn't receive any block slots for {latest_slot}");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            };

            let new_latest_slot = new_latest_slot.as_u64().unwrap();

            if latest_slot == new_latest_slot {
                warn!("No new slots for {latest_slot}");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }

            latest_slot = new_latest_slot;

            let mut total_time_to_index_millis = 0;
            let len = new_block_slots.len() as u128;

            for slot in new_block_slots {
                let instant = Instant::now();

                self.index_slot(slot.as_u64().unwrap(), commitment, transaction_details)
                    .await?;

                total_time_to_index_millis += instant.elapsed().as_millis();
            }

            info!(
                "Avg time to index {len} blocks {}",
                (total_time_to_index_millis / len)
            );
        }
    }
}
