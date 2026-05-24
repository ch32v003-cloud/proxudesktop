use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE_URL: &str = "https://proxu.pro/api/user";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiResponseProxy {
    pub id: String,
    pub ip: Option<String>,
    pub domain: Option<String>,
    pub port: Value,
    pub protocol: Option<String>,
    pub link: Option<String>,
    pub connection_string: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiVpnConfig {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub vpn_id: Option<Value>,
    #[serde(default)]
    pub server_id: Option<Value>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<Value>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub config: Option<String>,
    #[serde(default)]
    pub connection_string: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiUserProfile {
    pub balance: Value,
    pub email: Option<String>,
}

fn api_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .user_agent("ProxuDesktop/0.1")
        .build()
        .expect("Failed to build reqwest client")
}

fn auth_get(token: &str, url: String) -> reqwest::RequestBuilder {
    api_client()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
}

async fn json_response(res: reqwest::Response, context: &str) -> Result<Value, String> {
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed read {context} response body: {e}"))?;

    if !status.is_success() {
        return Err(format!("Server returned error code: {status}: {body}"));
    }

    serde_json::from_str::<Value>(&body).map_err(|e| format!("Failed parse {context} JSON: {e}"))
}

fn parse_vpn_value(value: Value) -> Vec<ApiVpnConfig> {
    let values: Vec<Value> = if let Some(items) = value.as_array() {
        items.clone()
    } else if let Some(items) = value.get("vpns").and_then(Value::as_array) {
        items.clone()
    } else if let Some(items) = value.get("vpn").and_then(Value::as_array) {
        items.clone()
    } else if let Some(items) = value.get("data").and_then(Value::as_array) {
        items.clone()
    } else if value.is_object() {
        vec![value]
    } else {
        Vec::new()
    };

    values
        .into_iter()
        .filter_map(|item| serde_json::from_value::<ApiVpnConfig>(item).ok())
        .collect()
}

pub async fn get_proxies(token: &str) -> Result<Vec<ApiResponseProxy>, String> {
    let url = format!("{BASE_URL}/proxies");
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    let value = json_response(res, "proxies").await?;
    serde_json::from_value::<Vec<ApiResponseProxy>>(value)
        .map_err(|e| format!("Failed parse proxies: {e}"))
}

pub async fn get_user_vpns(token: &str) -> Result<Vec<ApiVpnConfig>, String> {
    let url = format!("{BASE_URL}/vpn");
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    let value = json_response(res, "VPN configs").await?;
    Ok(parse_vpn_value(value))
}

pub async fn get_vpn_config(token: &str, vpn_id: &str) -> Result<ApiVpnConfig, String> {
    let url = format!("{BASE_URL}/vpn/{vpn_id}/config");
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    let value = json_response(res, "VPN config").await?;
    serde_json::from_value::<ApiVpnConfig>(value)
        .map_err(|e| format!("Failed parse VPN config: {e}"))
}

pub async fn get_profile(token: &str) -> Result<ApiUserProfile, String> {
    let url = format!("{BASE_URL}/profile");
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    let value = json_response(res, "profile details").await?;
    serde_json::from_value::<ApiUserProfile>(value)
        .map_err(|e| format!("Failed parse profile details: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_vpn_array_response() {
        let parsed = parse_vpn_value(json!([
            {
                "id": "vpn_1",
                "server_id": 7,
                "link": "vless://user@example.com:443"
            }
        ]));

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, json!("vpn_1"));
        assert_eq!(parsed[0].server_id, Some(json!(7)));
        assert_eq!(
            parsed[0].link.as_deref(),
            Some("vless://user@example.com:443")
        );
    }

    #[test]
    fn parse_vpn_object_with_vpns_array() {
        let parsed = parse_vpn_value(json!({
            "vpns": [
                {
                    "vpn_id": "vpn_2",
                    "config": "vmess://encoded"
                }
            ]
        }));

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].vpn_id, Some(json!("vpn_2")));
        assert_eq!(parsed[0].config.as_deref(), Some("vmess://encoded"));
    }

    #[test]
    fn parse_vpn_data_array_response() {
        let parsed = parse_vpn_value(json!({
            "data": [
                {
                    "id": 3,
                    "link": "vless://user@example.net:8443"
                }
            ]
        }));

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, json!(3));
    }

    #[test]
    fn parse_single_vpn_object_response() {
        let parsed = parse_vpn_value(json!({
            "id": "vpn_4",
            "link": "vless://user@example.org:443"
        }));

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, json!("vpn_4"));
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateProxyRequest {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub link: String,
    pub type_: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateProxyResponse {
    pub id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentRequest {
    pub amount: f64,
    pub currency: String,
    pub method: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentResponse {
    pub payment_id: String,
    pub status: String,
    pub payment_url: Option<String>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub amount: f64,
    pub currency: String,
    pub description: String,
    pub status: String,
    pub transaction_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<Transaction>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

fn auth_post(token: &str, url: String) -> reqwest::RequestBuilder {
    api_client()
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
}

fn auth_get_with_params(token: &str, url: String, params: Vec<(&str, String)>) -> reqwest::RequestBuilder {
    let mut request = api_client()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json");
    
    for (key, value) in params {
        request = request.query(&[(key, &value)]);
    }
    
    request
}

pub async fn create_proxy(token: &str, request: CreateProxyRequest) -> Result<CreateProxyResponse, String> {
    let url = format!("{}/proxies", BASE_URL);
    
    let res = auth_post(token, url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "create proxy").await?;
    serde_json::from_value::<CreateProxyResponse>(value)
        .map_err(|e| format!("Failed to parse create proxy response: {}", e))
}

pub async fn process_payment(token: &str, request: PaymentRequest) -> Result<PaymentResponse, String> {
    let url = format!("{}/payment", BASE_URL);
    
    let res = auth_post(token, url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "payment").await?;
    serde_json::from_value::<PaymentResponse>(value)
        .map_err(|e| format!("Failed to parse payment response: {}", e))
}

pub async fn get_transaction_history(
    token: &str,
    page: u32,
    per_page: u32,
) -> Result<TransactionHistoryResponse, String> {
    let url = format!("{}/transactions", BASE_URL);
    let params = vec![
        ("page", page.to_string()),
        ("per_page", per_page.to_string()),
    ];
    
    let res = auth_get_with_params(token, url, params)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "transactions").await?;
    serde_json::from_value::<TransactionHistoryResponse>(value)
        .map_err(|e| format!("Failed to parse transactions: {}", e))
}
