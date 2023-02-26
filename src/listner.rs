use std::sync::Arc;

use futures::future::join_all;
use log::{info, warn};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::{commitment_config::CommitmentConfig, slot_history::Slot};
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};
use tokio::time::Instant;

use crate::block_store::{BlockInformation, BlockStore};

#[derive(Clone)]
pub struct Listner {
    pub rpc_client: Arc<RpcClient>,
    pub block_store: BlockStore,
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
        //        let parent_slot = block.parent_slot;

        let Some(block_height) = block.block_height else {
            warn!("Received no block height for slot {slot} and blockhash {blockhash}");
            return Ok(());
        };

        //        let Some(transactions) = block.transactions else {
        //            warn!("No transactions in block");
        //            return Ok(());
        //        };

        let block_info = BlockInformation { slot, block_height };

        self.block_store
            .add_block(blockhash, block_info, commitment_config)
            .await;

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

        info!("Listening to blocks {commitment_config:?}");

        let mut slot_que = Vec::new();
        let mut last_new_slot_time = Instant::now();

        loop {
            let new_latest_slot = self
                .rpc_client
                .get_slot_with_commitment(commitment_config)
                .await?;

            if latest_slot == new_latest_slot {
                warn!("No new slots for {latest_slot}");
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                continue;
            }

            println!(
                "new latest slot {latest_slot} -> {new_latest_slot} in {:?}",
                last_new_slot_time.elapsed()
            );
            last_new_slot_time = Instant::now();

            let mut new_block_slots = (latest_slot..new_latest_slot).into_iter().collect();
            latest_slot = new_latest_slot;

            // need not queue up new slots if they are less than 16 and the queue is empty
            let slots_to_get_blocks =
                if slot_que.is_empty() && (new_latest_slot - latest_slot) <= 16 {
                    new_block_slots
                } else {
                    slot_que.append(&mut new_block_slots);
                    slot_que.split_off(slot_que.len().min(16))
                };

            let index_futs = slots_to_get_blocks
                .iter()
                .map(|slot| self.index_slot(*slot, commitment_config, transaction_details));

            for (index, status) in join_all(index_futs).await.into_iter().enumerate() {
                if status.is_err() {
                    slot_que.push(slots_to_get_blocks[index]);
                }
            }
        }
    }
}
