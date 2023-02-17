import { Connection, clusterApiUrl, Commitment, Finality } from "@solana/web3.js";

type TransactionDetails = 'accounts' | 'full' | 'none' | 'signatures';

class Listner {
    connection: Connection;

    constructor(connection: Connection) {
        this.connection = connection;
    }

    async indexSlot(slot: number, commitment: Finality, transaction_details: TransactionDetails) {
        const block = await this
            .connection
            .getBlock(
                slot,
                {
                    //@ts-ignore
                    transaction_details,
                    commitment,
                    rewards: false, // this can be turned on later
                    maxSupportedTransactionVersion: 0
                },
            );

        if (block == undefined) {
            console.log("No block found for ", slot);
            return;
        }

        let blockhash = block.blockhash;
        let parent_slot = block.parentSlot;

        if (block.transactions == undefined) {
            console.log("No transactions in block");
            return;
        };

        console.log(
            `${slot} at ${blockhash} with ${block.transactions.length} txs and parent ${parent_slot}`,
        );
    }

    async listen(commitment: Commitment, transaction_details: 'full') {
        let latest_slot = await this.connection.getSlot(commitment);

        console.log(`Listening to blocks ${commitment} with ${transaction_details} transaction details`);

        while (true) {
            const new_block_slots = await this.connection.getBlocks(latest_slot, undefined, commitment as Finality);
            const len = new_block_slots.length;

            if (len == 0) {
                console.warn("{latest_slot} No slots");
                //                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }

            const new_latest_slot = new_block_slots[len - 1];

            if (latest_slot == new_latest_slot) {
                console.warn(`No new slots for ${latest_slot}`);
                //tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }

            latest_slot = new_latest_slot;

            let total_time_to_index_millis = 0;

            for (const slot of new_block_slots) {
                let instant = Date.now();

                await this.indexSlot(slot, commitment as Finality, transaction_details);

                total_time_to_index_millis += (Date.now() - instant);
            }

            console.log(
                `Avg time to index ${len} blocks ${(total_time_to_index_millis / len)}`,
            );
        }
    }

}

(async () => {
    const rpc_addr = process.argv[2] || clusterApiUrl('mainnet-beta', true);

    console.log(rpc_addr);

    const connection = new Connection(rpc_addr);
    const listner = new Listner(connection);

    await listner.listen('finalized', 'full');
})()
