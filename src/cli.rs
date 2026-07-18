use crate::commands;
use crate::error::AppError;
use crate::rpc::RpcClient;
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[command(name = "btc-cli")]
#[command(about = "A command-line tool for interacting with a Bitcoin Core node via JSON-RPC.", long_about = None)]
pub struct Cli {
    #[arg(long, help = "RPC URL of the Bitcoin node")]
    pub rpc_url: Option<String>,

    #[arg(long, help = "RPC username")]
    pub rpc_user: Option<String>,

    #[arg(long, help = "RPC password")]
    pub rpc_password: Option<String>,

    #[arg(long, help = "Name of the wallet to use")]
    pub wallet: Option<String>,

    #[arg(long, help = "Path to TOML configuration file")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(about = "Display general blockchain information")]
    BlockchainInfo,

    #[command(about = "Display wallet information")]
    WalletInfo,

    #[command(about = "Print the wallet balance")]
    Balance {
        #[arg(long, help = "Include unconfirmed balance in the output")]
        include_unconfirmed: bool,
    },

    #[command(about = "Generate and print a new receiving address")]
    NewAddress {
        #[arg(long, help = "Label for the address")]
        label: Option<String>,

        #[arg(long, help = "Address type (legacy, p2sh-segwit, bech32)")]
        address_type: Option<String>,
    },

    #[command(about = "Execute an arbitrary Bitcoin Core RPC method")]
    Rpc {
        #[arg(help = "The RPC method name")]
        method: String,

        #[arg(num_args = 0.., help = "Arguments for the RPC method")]
        params: Vec<String>,
    },
}

fn coerce_param(param: &str) -> serde_json::Value {
    match serde_json::from_str::<serde_json::Value>(param) {
        Ok(val) => val,
        Err(_) => serde_json::Value::String(param.to_string()),
    }
}

pub async fn run(cli: Cli, client: &RpcClient) -> Result<(), AppError> {
    match cli.command {
        Commands::BlockchainInfo => {
            let info = commands::blockchain::fetch(client).await?;
            commands::blockchain::print(&info);
        }
        Commands::WalletInfo => {
            let info = commands::wallet::info(client).await?;
            commands::wallet::print_info(&info);
        }
        Commands::Balance {
            include_unconfirmed,
        } => {
            let (confirmed, unconfirmed) =
                commands::wallet::balance(client, include_unconfirmed).await?;
            commands::wallet::print_balance(confirmed, unconfirmed);
        }
        Commands::NewAddress {
            label,
            address_type,
        } => {
            let addr =
                commands::address::new_address(client, label.as_deref(), address_type.as_deref())
                    .await?;
            println!("{}", addr);
        }
        Commands::Rpc { method, params } => {
            let coerced_params: Vec<serde_json::Value> =
                params.iter().map(|p| coerce_param(p)).collect();
            let json_params = serde_json::Value::Array(coerced_params);
            let response = client.call_raw(&method, json_params).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&response).map_err(AppError::Parse)?
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coerce_param() {
        use serde_json::json;

        // Number
        assert_eq!(coerce_param("200"), json!(200));
        assert_eq!(coerce_param("-5"), json!(-5));
        assert_eq!(coerce_param("1.23"), json!(1.23));

        // Boolean
        assert_eq!(coerce_param("true"), json!(true));
        assert_eq!(coerce_param("false"), json!(false));

        // JSON string with quotes
        assert_eq!(coerce_param(r#""hello""#), json!("hello"));

        // Plain string without quotes
        assert_eq!(coerce_param("hello"), json!("hello"));

        // Array
        assert_eq!(coerce_param("[1,2,3]"), json!([1, 2, 3]));

        // Object
        assert_eq!(coerce_param(r#"{"a": 1}"#), json!({"a": 1}));

        // Null
        assert_eq!(coerce_param("null"), json!(null));
    }
}
