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
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub remarks: Option<String>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VpnServer {
    pub id: Value,
    pub name: Option<String>,
    pub location: Option<String>,
    pub ip: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VpnInbound {
    pub id: i64,
    pub protocol: Option<String>,
    pub port: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutoCreateProfileResponse {
    pub id: String,
    pub link: String,
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatePaymentRequest {
    pub amount: f64,
    pub payment_method: String,
    pub client_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatePaymentResponse {
    pub payment_id: Option<String>,
    pub id: Option<String>,
    pub payment_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentStatusResponse {
    pub status: Option<String>,
    pub payment_id: Option<String>,
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

pub async fn get_vpn_servers(token: &str) -> Result<Vec<VpnServer>, String> {
    let url = format!("{}/vpn-servers", BASE_URL);
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "VPN servers").await?;
    serde_json::from_value::<Vec<VpnServer>>(value)
        .map_err(|e| format!("Failed to parse VPN servers: {}", e))
}

pub async fn get_vpn_inbounds(token: &str, server_id: &str) -> Result<Vec<VpnInbound>, String> {
    let url = format!("{}/vpn-inbounds/{}", BASE_URL, server_id);
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "VPN inbounds").await?;
    serde_json::from_value::<Vec<VpnInbound>>(value)
        .map_err(|e| format!("Failed to parse VPN inbounds: {}", e))
}

pub async fn auto_create_profile(token: &str) -> Result<AutoCreateProfileResponse, String> {
    eprintln!("[API] auto_create_profile: fetching VPN servers...");
    let servers = get_vpn_servers(token).await?;
    eprintln!("[API] auto_create_profile: got {} servers", servers.len());
    for s in &servers {
        eprintln!("[API]   server: id={:?}, name={:?}, location={:?}", s.id, s.name, s.location);
    }
    if servers.is_empty() {
        return Err("No VPN servers available".to_string());
    }

    let mut available: Vec<(VpnServer, VpnInbound)> = Vec::new();
    for server in &servers {
        let sid = match &server.id {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => {
                eprintln!("[API]   skipping server with non-string/number id: {:?}", server.id);
                continue;
            }
        };
        eprintln!("[API] auto_create_profile: fetching inbounds for server {}", sid);
        let inbounds = get_vpn_inbounds(token, &sid).await.unwrap_or_else(|e| {
            eprintln!("[API]   failed to get inbounds for server {}: {}", sid, e);
            Vec::new()
        });
        eprintln!("[API]   got {} inbounds for server {}", inbounds.len(), sid);
        for inbound in &inbounds {
            let proto = inbound.protocol.as_deref().unwrap_or("").to_lowercase();
            eprintln!("[API]     inbound: id={}, protocol={}, port={:?}", inbound.id, proto, inbound.port);
            if proto == "vless" {
                available.push((server.clone(), inbound.clone()));
            }
        }
    }

    if available.is_empty() {
        eprintln!("[API] auto_create_profile: no VLESS inbounds available");
        return Err("No available VLESS inbounds found on any server".to_string());
    }
    eprintln!("[API] auto_create_profile: {} VLESS inbounds available", available.len());

    use rand::Rng;
    let (selected_server, selected_inbound) = {
        let idx = rand::thread_rng().gen_range(0..available.len());
        available.remove(idx)
    };
    eprintln!("[API] auto_create_profile: selected server={:?}, inbound={}", selected_server.name, selected_inbound.id);

    let body = serde_json::json!({
        "type": "vpn",
        "quantity": 1,
        "xui_server_id": selected_server.id,
        "xui_inbound_id": selected_inbound.id,
    });
    eprintln!("[API] auto_create_profile: POST body: {}", body);

    let url = format!("{}/proxies", BASE_URL);
    let res = auth_post(token, url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "create profile").await?;
    eprintln!("[API] auto_create_profile: response: {}", value);

    // Response wraps proxy in {"proxies": [{...}]}
    let proxy_obj = value.get("proxies")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .unwrap_or(&value);

    let proxy_id = proxy_obj.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let link = proxy_obj.get("link")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if link.is_empty() || proxy_id.is_empty() {
        return Err(format!("Server returned incomplete response: {}", value));
    }

    // Extract name from VLESS link fragment (#...), decode percent-encoded chars
    let name = url::Url::parse(&link)
        .ok()
        .and_then(|u| {
            let fragment = u.fragment()?;
            Some(urlencoding::decode(fragment).unwrap_or_else(|_| std::borrow::Cow::Borrowed(fragment)).to_string())
        })
        .unwrap_or_else(|| {
            selected_server.name.clone().unwrap_or_else(|| {
                proxy_obj.get("domain")
                    .and_then(|v| v.as_str())
                    .unwrap_or("VPN")
                    .to_string()
            })
        });

    let server = proxy_obj.get("domain")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            url::Url::parse(&link)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| "unknown".to_string())
        });
    let port = proxy_obj.get("port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
        .unwrap_or_else(|| {
            url::Url::parse(&link)
                .ok()
                .and_then(|u| u.port())
                .unwrap_or(443)
        });

    eprintln!("[API] auto_create_profile: success: id={}, name={}, server={}", proxy_id, name, server);

    eprintln!("[API] auto_create_profile: success: id={}, name={}, server={}", proxy_id, name, server);

    Ok(AutoCreateProfileResponse {
        id: proxy_id,
        link,
        name,
        server,
        port,
        protocol: "vless".to_string(),
    })
}

pub async fn create_payment(token: &str, amount: f64, payment_method: &str) -> Result<CreatePaymentResponse, String> {
    eprintln!("[API] create_payment: amount={}, method={}", amount, payment_method);
    let url = format!("{}/payments/create", BASE_URL);
    let body = CreatePaymentRequest {
        amount,
        payment_method: payment_method.to_string(),
        client_type: "mobile".to_string(),
    };
    eprintln!("[API] create_payment: POST body: {}", serde_json::to_string(&body).unwrap_or_default());

    let res = auth_post(token, url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "create payment").await?;
    eprintln!("[API] create_payment: response: {}", value);

    let payment_id = value.get("payment_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| value.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let id = value.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let payment_url = value.get("payment_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let status = value.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());

    eprintln!("[API] create_payment: parsed: payment_id={:?}, payment_url={:?}, status={:?}", payment_id, payment_url, status);

    Ok(CreatePaymentResponse {
        payment_id,
        id,
        payment_url,
        status,
    })
}

pub async fn get_payment_status(token: &str, payment_id: &str) -> Result<PaymentStatusResponse, String> {
    let url = format!("{}/payments/{}/status", BASE_URL, payment_id);
    let res = auth_get(token, url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let value = json_response(res, "payment status").await?;
    let status = value.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());
    let pid = value.get("payment_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    Ok(PaymentStatusResponse {
        status,
        payment_id: pid,
    })
}
