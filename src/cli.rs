use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value_t = String::from("https://api.mainnet-beta.solana.com"))]
    pub rpc_addr: String,
    #[arg(short, long)]
    pub use_reqwest: bool,
}
