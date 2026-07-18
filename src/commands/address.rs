use crate::error::AppError;
use crate::rpc::RpcClient;
use serde_json::json;

pub fn build_params(label: Option<&str>, address_type: Option<&str>) -> serde_json::Value {
    match (label, address_type) {
        (Some(l), Some(t)) => json!([l, t]),
        (Some(l), None) => json!([l]),
        (None, Some(t)) => json!(["", t]),
        (None, None) => json!([]),
    }
}

pub async fn new_address(
    client: &RpcClient,
    label: Option<&str>,
    address_type: Option<&str>,
) -> Result<String, AppError> {
    let params = build_params(label, address_type);
    client.call("getnewaddress", params).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_params() {
        // Both None
        assert_eq!(build_params(None, None), json!([]));

        // Only label
        assert_eq!(build_params(Some("my-label"), None), json!(["my-label"]));

        // Only address_type
        assert_eq!(build_params(None, Some("bech32")), json!(["", "bech32"]));

        // Both provided
        assert_eq!(
            build_params(Some("my-label"), Some("bech32")),
            json!(["my-label", "bech32"])
        );
    }
}
