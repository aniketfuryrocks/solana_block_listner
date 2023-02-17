use log::{info, warn};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::{commitment_config::CommitmentConfig, slot_history::Slot};
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};
use tokio::time::Instant;

pub struct Listner {
    pub rpc_client: RpcClient,
}

impl Listner {
    pub async fn index_slot(
        &self,
        slot: Slot,
        commitment_config: CommitmentConfig,
        transaction_details: TransactionDetails,
    ) -> anyhow::Result<()> {
        let block = self
            .rpc_client
            .get_block_with_config(
                slot,
                RpcBlockConfig {
                    transaction_details: Some(transaction_details),
                    commitment: Some(commitment_config),
                    max_supported_transaction_version: Some(0),
                    encoding: Some(UiTransactionEncoding::Base64),
                    rewards: None, // this can be turned on later
                },
            )
            .await?;

        let blockhash = block.blockhash;
        let parent_slot = block.parent_slot;

        let Some(block_height) = block.block_height else {
            warn!("Received no block height for slot {slot} and blockhash {blockhash}");
            return Ok(());
        };

        let Some(transactions) = block.transactions else {
            warn!("No transactions in block");
            return Ok(());
        };

        info!(
            "{slot} at {blockhash} height {block_height} with {} txs and parent {parent_slot}",
            transactions.len()
        );

        Ok(())
    }

    pub async fn listen(
        self,
        commitment_config: CommitmentConfig,
        transaction_details: TransactionDetails,
    ) -> anyhow::Result<()> {
        let mut latest_slot = self
            .rpc_client
            .get_slot_with_commitment(commitment_config)
            .await?;

        info!("Listening to blocks {commitment_config:?} with {transaction_details:?} transaction details");

        loop {
            let new_block_slots = self
                .rpc_client
                .get_blocks_with_commitment(latest_slot, None, commitment_config)
                .await?;

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

                self.index_slot(slot, commitment_config, transaction_details)
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
