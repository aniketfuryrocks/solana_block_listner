use std::sync::Arc;

use anyhow::bail;
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::TransactionDetails;

use self::listner::Listner;
use self::reqwest_listner::ReqwestListner;
use cli::Args;

mod cli;
mod listner;
mod reqwest_listner;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let Args {
        rpc_addr,
        use_reqwest,
    } = Args::parse();

    if use_reqwest {
        let listner = ReqwestListner {
            rpc_client: reqwest::ClientBuilder::new().build()?,
            rpc_addr,
        };

        listner.listen("finalized", "full").await?;
    } else {
        let listner = Listner {
            rpc_client: Arc::new(RpcClient::new(rpc_addr)),
        };

        listner
            .listen(CommitmentConfig::finalized(), TransactionDetails::Full)
            .await?;
    }

    bail!("Listener exited")
}
