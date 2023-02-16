use anyhow::bail;
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::TransactionDetails;

use self::listner::Listner;
use cli::Args;

mod cli;
mod listner;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let Args { rpc_addr } = Args::parse();

    let listner = Listner {
        rpc_client: RpcClient::new(rpc_addr),
    };

    listner
        .listen(CommitmentConfig::finalized(), TransactionDetails::Full)
        .await?;

    bail!("Listener exited")
}
