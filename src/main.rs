mod cli;
mod commands;
mod config;
mod error;
mod rpc;

use clap::Parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();
    if let Err(e) = run(cli).await {
        tracing::error!("Application error: {}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(cli: cli::Cli) -> Result<(), error::AppError> {
    let partial_cfg = config::PartialConfig {
        rpc_url: cli.rpc_url.clone(),
        rpc_user: cli.rpc_user.clone(),
        rpc_password: cli.rpc_password.clone(),
        wallet: cli.wallet.clone(),
    };

    let cfg = config::resolve_from_sources(&partial_cfg, cli.config.as_deref())?;
    let client = rpc::RpcClient::new(&cfg);
    cli::run(cli, &client).await
}
