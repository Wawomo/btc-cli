use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    MissingRequired(String),
    FileRead {
        path: String,
        source: std::io::Error,
    },
    TomlParse(toml::de::Error),
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::FileRead { source, .. } => Some(source),
            ConfigError::TomlParse(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingRequired(fields) => {
                write!(f, "missing required configuration: {}", fields)
            }
            ConfigError::FileRead { path, source } => {
                write!(f, "failed to read config file '{}': {}", path, source)
            }
            ConfigError::TomlParse(e) => write!(f, "failed to parse config TOML: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Connection { url: String, source: reqwest::Error },
    Auth { url: String },
    Rpc { code: i64, message: String },
    Wallet(String),
    Parse(serde_json::Error),
    Http { status: reqwest::StatusCode, body: String },
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Config(e) => Some(e),
            AppError::Connection { source, .. } => Some(source),
            AppError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(e) => write!(f, "{}", e),
            AppError::Connection { url, source } => {
                let err_str = source.to_string().to_lowercase();
                if err_str.contains("connection refused") || err_str.contains("connect error") {
                    write!(f, "could not reach node at {} — connection refused", url)
                } else {
                    write!(f, "could not reach node at {}: {}", url, source)
                }
            }
            AppError::Auth { url } => write!(
                f,
                "authentication failed for {} — check RPC user/password",
                url
            ),
            AppError::Wallet(msg) => write!(f, "wallet error: {}", msg),
            AppError::Parse(e) => write!(f, "failed to parse response: {}", e),
            AppError::Http { status, body } => {
                write!(f, "HTTP error ({}): {}", status, body.trim())
            }
            AppError::Rpc { code, message } => match *code {
                -32601 => write!(f, "RPC error -32601: Method not found"),
                -32602 => write!(f, "RPC error -32602: invalid parameters"),
                -8 => write!(f, "RPC error -8: {}", message),
                -18 => write!(
                    f,
                    "no wallet is loaded — run 'btc-cli rpc loadwallet <name>' or 'btc-cli rpc createwallet <name>'"
                ),
                -28 => write!(f, "RPC error -28: node not ready yet"),
                _ => write!(f, "RPC error {}: {}", code, message),
            },
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::TomlParse(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_missing_required_display() {
        let err = ConfigError::MissingRequired("rpc_user, rpc_password".to_string());
        assert_eq!(
            err.to_string(),
            "missing required configuration: rpc_user, rpc_password"
        );
    }

    #[test]
    fn test_auth_error_display() {
        let err = AppError::Auth {
            url: "http://127.0.0.1:18443".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "authentication failed for http://127.0.0.1:18443 — check RPC user/password"
        );
    }

    #[test]
    fn test_rpc_method_not_found_display() {
        let err = AppError::Rpc {
            code: -32601,
            message: "Method not found".to_string(),
        };
        assert_eq!(err.to_string(), "RPC error -32601: Method not found");
    }

    #[test]
    fn test_rpc_wallet_not_found_display() {
        let err = AppError::Rpc {
            code: -18,
            message: "Wallet not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "no wallet is loaded — run 'btc-cli rpc loadwallet <name>' or 'btc-cli rpc createwallet <name>'"
        );
    }
}
