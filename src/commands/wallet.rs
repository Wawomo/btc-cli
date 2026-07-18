use crate::error::AppError;
use crate::rpc::RpcClient;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct WalletInfo {
    pub walletname: String,
    #[serde(default)]
    pub balance: f64,
    #[serde(default)]
    pub unconfirmed_balance: f64,
    pub txcount: u64,
}

pub async fn info(client: &RpcClient) -> Result<WalletInfo, AppError> {
    let mut w_info: WalletInfo = client.call("getwalletinfo", serde_json::json!([])).await?;
    
    // On newer Bitcoin Core versions (v21+), getwalletinfo does not return balance fields.
    // In that case, they will default to 0.0. We query getbalances to fetch the actual balances.
    #[derive(Deserialize)]
    struct Balances {
        mine: MineBalances,
    }
    #[derive(Deserialize)]
    struct MineBalances {
        trusted: f64,
        untrusted_pending: f64,
    }
    
    if let Ok(bal_resp) = client.call::<Balances>("getbalances", serde_json::json!([])).await {
        w_info.balance = bal_resp.mine.trusted;
        w_info.unconfirmed_balance = bal_resp.mine.untrusted_pending;
    }
    
    Ok(w_info)
}

pub async fn balance(
    client: &RpcClient,
    include_unconfirmed: bool,
) -> Result<(f64, Option<f64>), AppError> {
    if include_unconfirmed {
        let w_info = info(client).await?;
        Ok((w_info.balance, Some(w_info.unconfirmed_balance)))
    } else {
        let bal: f64 = client.call("getbalance", serde_json::json!([])).await?;
        Ok((bal, None))
    }
}

pub fn print_info(info: &WalletInfo) {
    println!("{:<22}{}", "Wallet:", info.walletname);
    println!("{:<22}{:.8} BTC", "Balance:", info.balance);
    println!(
        "{:<22}{:.8} BTC",
        "Unconfirmed balance:", info.unconfirmed_balance
    );
    println!("{:<22}{}", "Transactions:", info.txcount);
}

pub fn print_balance(confirmed: f64, unconfirmed: Option<f64>) {
    match unconfirmed {
        Some(unconf) => {
            println!("{:<14}{:.8} BTC", "Confirmed:", confirmed);
            println!("{:<14}{:.8} BTC", "Unconfirmed:", unconf);
        }
        None => {
            println!("{:.8} BTC", confirmed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_wallet_info_deserialization() {
        let json_data = r#"{
            "walletname": "wallet1",
            "walletversion": 169900,
            "balance": 50.00000000,
            "unconfirmed_balance": 0.00000000,
            "immature_balance": 0.00000000,
            "txcount": 3,
            "keypoololdest": 1506075353,
            "keypoolsize": 1000,
            "hdseedid": "..."
        }"#;

        let parsed: WalletInfo = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            parsed,
            WalletInfo {
                walletname: "wallet1".to_string(),
                balance: 50.00000000,
                unconfirmed_balance: 0.00000000,
                txcount: 3,
            }
        );
    }

    #[tokio::test]
    async fn test_wallet_not_found_mapping() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "result": null,
            "error": {
                "code": -18,
                "message": "Requested wallet does not exist or is not loaded"
            },
            "id": "btc-cli"
        });

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let cfg = crate::config::Config {
            rpc_url: mock_server.uri(),
            rpc_user: "user".to_string(),
            rpc_password: "pass".to_string(),
            wallet: Some("invalid-wallet".to_string()),
        };

        let client = RpcClient::new(&cfg);
        let res = info(&client).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            AppError::Wallet(msg) => {
                assert!(msg.contains("no wallet is loaded"));
            }
            other => panic!("expected AppError::Wallet, got {:?}", other),
        }
    }
}
