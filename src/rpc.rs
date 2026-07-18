use crate::config::Config;
use crate::error::AppError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub struct RpcClient {
    http: reqwest::Client,
    url: String,
    user: String,
    password: String,
    wallet: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RpcResponseEnvelope {
    result: Value,
    error: Option<RpcErrorBody>,
    #[serde(rename = "id")]
    _id: Value,
}

#[derive(Deserialize, Debug)]
struct RpcErrorBody {
    code: i64,
    message: String,
}

impl RpcClient {
    pub fn new(cfg: &Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: cfg.rpc_url.clone(),
            user: cfg.rpc_user.clone(),
            password: cfg.rpc_password.clone(),
            wallet: cfg.wallet.clone(),
        }
    }

    pub fn endpoint(&self, method: &str) -> String {
        let is_node_command = matches!(
            method,
            "createwallet" | "loadwallet" | "unloadwallet" | "listwallets" | "listwalletdir"
        );
        match &self.wallet {
            Some(w) if !is_node_command => {
                let trimmed = self.url.trim_end_matches('/');
                format!("{}/wallet/{}", trimmed, w)
            }
            _ => self.url.clone(),
        }
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, AppError> {
        let raw = self.call_raw(method, params).await?;
        let parsed = serde_json::from_value::<T>(raw)?;
        Ok(parsed)
    }

    pub async fn call_raw(&self, method: &str, params: Value) -> Result<Value, AppError> {
        let endpoint = self.endpoint(method);
        let payload = serde_json::json!({
            "jsonrpc": "1.0",
            "id": "btc-cli",
            "method": method,
            "params": params,
        });

        let start = std::time::Instant::now();
        tracing::debug!(method = method, params = ?params, endpoint = %endpoint, "Sending JSON-RPC request");
        tracing::info!(method = method, endpoint = %endpoint, "Sending JSON-RPC request");

        let response = self
            .http
            .post(&endpoint)
            .basic_auth(&self.user, Some(&self.password))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(method = method, error = ?e, "Connection failure");
                AppError::Connection {
                    url: self.url.clone(),
                    source: e,
                }
            })?;

        let status = response.status();
        tracing::info!(
            method = method,
            status = status.as_u16(),
            duration_ms = start.elapsed().as_millis(),
            "Received HTTP response"
        );

        if status == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(url = %self.url, "Authentication failed (401 Unauthorized)");
            return Err(AppError::Auth {
                url: self.url.clone(),
            });
        }

        let body_text = response.text().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to read response body");
            AppError::Connection {
                url: self.url.clone(),
                source: e,
            }
        })?;

        let envelope: RpcResponseEnvelope = match serde_json::from_str(&body_text) {
            Ok(env) => env,
            Err(e) => {
                if !status.is_success() && status.as_u16() != 500 {
                    tracing::error!(status = status.as_u16(), body = %body_text, "HTTP protocol error");
                    return Err(AppError::Http {
                        status,
                        body: body_text,
                    });
                }
                tracing::error!(error = ?e, body = %body_text, "Failed to parse JSON response");
                return Err(AppError::Parse(e));
            }
        };

        if let Some(err_body) = envelope.error {
            tracing::warn!(code = err_body.code, message = %err_body.message, "RPC error returned from node");
            if err_body.code == -18 {
                return Err(AppError::Wallet(
                    "no wallet is loaded — run 'btc-cli rpc loadwallet <name>' or 'btc-cli rpc createwallet <name>'".to_string()
                ));
            }
            return Err(AppError::Rpc {
                code: err_body.code,
                message: err_body.message,
            });
        }

        tracing::debug!(result = ?envelope.result, "RPC call succeeded");
        Ok(envelope.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_rpc_success() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "result": "success-value",
            "error": null,
            "id": "btc-cli"
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let cfg = Config {
            rpc_url: mock_server.uri(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: None,
        };

        let client = RpcClient::new(&cfg);
        let res: String = client
            .call("testmethod", serde_json::json!([]))
            .await
            .unwrap();
        assert_eq!(res, "success-value");
    }

    #[tokio::test]
    async fn test_rpc_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let cfg = Config {
            rpc_url: mock_server.uri(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: None,
        };

        let client = RpcClient::new(&cfg);
        let res: Result<String, AppError> = client.call("testmethod", serde_json::json!([])).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            AppError::Auth { url } => assert_eq!(url, mock_server.uri()),
            other => panic!("expected Auth error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rpc_error_method_not_found() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "result": null,
            "error": {
                "code": -32601,
                "message": "Method not found"
            },
            "id": "btc-cli"
        });

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let cfg = Config {
            rpc_url: mock_server.uri(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: None,
        };

        let client = RpcClient::new(&cfg);
        let res: Result<String, AppError> = client.call("testmethod", serde_json::json!([])).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            AppError::Rpc { code, message } => {
                assert_eq!(code, -32601);
                assert_eq!(message, "Method not found");
            }
            other => panic!("expected Rpc error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rpc_connection_error() {
        let cfg = Config {
            rpc_url: "http://127.0.0.1:59999".to_string(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: None,
        };

        let client = RpcClient::new(&cfg);
        let res: Result<String, AppError> = client.call("testmethod", serde_json::json!([])).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            AppError::Connection { url, .. } => assert_eq!(url, "http://127.0.0.1:59999"),
            other => panic!("expected Connection error, got {:?}", other),
        }
    }

    #[test]
    fn test_endpoint_routing() {
        let cfg = Config {
            rpc_url: "http://127.0.0.1:18443".to_string(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: Some("mywallet".to_string()),
        };
        let client = RpcClient::new(&cfg);

        // Wallet commands should use wallet endpoint
        assert_eq!(client.endpoint("getwalletinfo"), "http://127.0.0.1:18443/wallet/mywallet");
        assert_eq!(client.endpoint("getnewaddress"), "http://127.0.0.1:18443/wallet/mywallet");

        // Node level / wallet management commands should use base endpoint
        assert_eq!(client.endpoint("loadwallet"), "http://127.0.0.1:18443");
        assert_eq!(client.endpoint("createwallet"), "http://127.0.0.1:18443");
        assert_eq!(client.endpoint("listwallets"), "http://127.0.0.1:18443");
        assert_eq!(client.endpoint("unloadwallet"), "http://127.0.0.1:18443");
    }

    #[tokio::test]
    async fn test_rpc_http_protocol_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
            .mount(&mock_server)
            .await;

        let cfg = Config {
            rpc_url: mock_server.uri(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: None,
        };

        let client = RpcClient::new(&cfg);
        let res: Result<String, AppError> = client.call("testmethod", serde_json::json!([])).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            AppError::Http { status, body } => {
                assert_eq!(status, reqwest::StatusCode::BAD_GATEWAY);
                assert_eq!(body, "Bad Gateway");
            }
            other => panic!("expected Http error, got {:?}", other),
        }
    }
}
