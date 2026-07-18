use crate::error::AppError;
use crate::rpc::RpcClient;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub difficulty: f64,
    pub verificationprogress: f64,
}

pub async fn fetch(client: &RpcClient) -> Result<BlockchainInfo, AppError> {
    client
        .call("getblockchaininfo", serde_json::json!([]))
        .await
}

pub fn print(info: &BlockchainInfo) {
    println!("{:<22}{}", "Chain:", info.chain);
    println!("{:<22}{}", "Blocks:", info.blocks);
    println!("{:<22}{}", "Headers:", info.headers);
    println!("{:<22}{}", "Difficulty:", info.difficulty);
    println!(
        "{:<22}{:.2}%",
        "Verification progress:",
        info.verificationprogress * 100.0
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_info_deserialization() {
        let json_data = r#"{
            "chain": "regtest",
            "blocks": 12,
            "headers": 12,
            "difficulty": 4.656542373906925e-10,
            "verificationprogress": 1.0,
            "bestblockhash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
        }"#;

        let parsed: BlockchainInfo = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            parsed,
            BlockchainInfo {
                chain: "regtest".to_string(),
                blocks: 12,
                headers: 12,
                difficulty: 4.6565423739069247e-10,
                verificationprogress: 1.0,
            }
        );
    }
}
