use crate::error::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
    pub wallet: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PartialConfig {
    pub rpc_url: Option<String>,
    pub rpc_user: Option<String>,
    pub rpc_password: Option<String>,
    pub wallet: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileConfig {
    pub rpc_url: Option<String>,
    pub rpc_user: Option<String>,
    pub rpc_password: Option<String>,
    pub wallet: Option<String>,
}

pub fn resolve(
    cli: &PartialConfig,
    env_lookup: fn(&str) -> Option<String>,
    file_contents: Option<&str>,
) -> Result<Config, ConfigError> {
    // 1. Parse file config if provided
    let file_cfg = if let Some(contents) = file_contents {
        toml::from_str::<FileConfig>(contents)?
    } else {
        FileConfig::default()
    };

    // 2. Resolve each field: CLI > Env > File > Default
    let rpc_url = cli
        .rpc_url
        .clone()
        .or_else(|| env_lookup("BTC_RPC_URL"))
        .or(file_cfg.rpc_url)
        .unwrap_or_else(|| "http://127.0.0.1:18443".to_string());

    let rpc_user = cli
        .rpc_user
        .clone()
        .or_else(|| env_lookup("BTC_RPC_USER"))
        .or(file_cfg.rpc_user);

    let rpc_password = cli
        .rpc_password
        .clone()
        .or_else(|| env_lookup("BTC_RPC_PASSWORD"))
        .or(file_cfg.rpc_password);

    let wallet = cli
        .wallet
        .clone()
        .or_else(|| env_lookup("BTC_RPC_WALLET"))
        .or(file_cfg.wallet);

    // 3. Validate required fields
    let mut missing = Vec::new();
    if rpc_user.is_none() {
        missing.push("rpc_user");
    }
    if rpc_password.is_none() {
        missing.push("rpc_password");
    }

    if !missing.is_empty() {
        return Err(ConfigError::MissingRequired(missing.join(", ")));
    }

    tracing::info!("Resolved configuration: URL={}, User={}, Password=[REDACTED], Wallet={:?}", 
        rpc_url, 
        rpc_user.as_deref().unwrap_or(""), 
        wallet
    );

    Ok(Config {
        rpc_url,
        rpc_user: rpc_user.unwrap(),
        rpc_password: rpc_password.unwrap(),
        wallet,
    })
}

pub fn resolve_from_sources(
    cli: &PartialConfig,
    config_path: Option<&Path>,
) -> Result<Config, crate::error::AppError> {
    let default_path = Path::new("config.toml");
    let path_to_read = match config_path {
        Some(p) => Some(p),
        None => {
            if default_path.exists() {
                Some(default_path)
            } else {
                None
            }
        }
    };

    let file_contents = match path_to_read {
        Some(p) => {
            tracing::info!("Reading configuration file: {:?}", p);
            let contents = fs::read_to_string(p).map_err(|e| ConfigError::FileRead {
                path: p.to_string_lossy().into_owned(),
                source: e,
            })?;
            Some(contents)
        }
        None => {
            tracing::debug!("No configuration file found or specified");
            None
        }
    };

    let env_lookup: fn(&str) -> Option<String> = |key| std::env::var(key).ok();

    let cfg = resolve(cli, env_lookup, file_contents.as_deref())?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_wins_over_all() {
        let cli = PartialConfig {
            rpc_url: Some("http://cli:18443".to_string()),
            rpc_user: Some("cliuser".to_string()),
            rpc_password: Some("clipass".to_string()),
            wallet: Some("cliwallet".to_string()),
        };

        let env_lookup: fn(&str) -> Option<String> = |key| match key {
            "BTC_RPC_URL" => Some("http://env:18443".to_string()),
            "BTC_RPC_USER" => Some("envuser".to_string()),
            "BTC_RPC_PASSWORD" => Some("envpass".to_string()),
            "BTC_RPC_WALLET" => Some("envwallet".to_string()),
            _ => None,
        };

        let file_contents = r#"
            rpc_url = "http://file:18443"
            rpc_user = "fileuser"
            rpc_password = "filepass"
            wallet = "filewallet"
        "#;

        let resolved = resolve(&cli, env_lookup, Some(file_contents)).unwrap();
        assert_eq!(
            resolved,
            Config {
                rpc_url: "http://cli:18443".to_string(),
                rpc_user: "cliuser".to_string(),
                rpc_password: "clipass".to_string(),
                wallet: Some("cliwallet".to_string()),
            }
        );
    }

    #[test]
    fn test_env_wins_over_file() {
        let cli = PartialConfig::default();

        let env_lookup: fn(&str) -> Option<String> = |key| match key {
            "BTC_RPC_URL" => Some("http://env:18443".to_string()),
            "BTC_RPC_USER" => Some("envuser".to_string()),
            "BTC_RPC_PASSWORD" => Some("envpass".to_string()),
            _ => None,
        };

        let file_contents = r#"
            rpc_url = "http://file:18443"
            rpc_user = "fileuser"
            rpc_password = "filepass"
        "#;

        let resolved = resolve(&cli, env_lookup, Some(file_contents)).unwrap();
        assert_eq!(
            resolved,
            Config {
                rpc_url: "http://env:18443".to_string(),
                rpc_user: "envuser".to_string(),
                rpc_password: "envpass".to_string(),
                wallet: None,
            }
        );
    }

    #[test]
    fn test_file_wins_over_default() {
        let cli = PartialConfig::default();
        let env_lookup: fn(&str) -> Option<String> = |_| None;
        let file_contents = r#"
            rpc_user = "fileuser"
            rpc_password = "filepass"
        "#;

        let resolved = resolve(&cli, env_lookup, Some(file_contents)).unwrap();
        assert_eq!(
            resolved,
            Config {
                rpc_url: "http://127.0.0.1:18443".to_string(),
                rpc_user: "fileuser".to_string(),
                rpc_password: "filepass".to_string(),
                wallet: None,
            }
        );
    }

    #[test]
    fn test_missing_required_fields() {
        let cli = PartialConfig::default();
        let env_lookup: fn(&str) -> Option<String> = |_| None;
        let file_contents = "";

        let result = resolve(&cli, env_lookup, Some(file_contents));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "missing required configuration: rpc_user, rpc_password"
        );
    }
}
