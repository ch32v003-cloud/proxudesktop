use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://proxu.pro/api/user";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiResponseProxy {
    pub id: String,
    pub ip: Option<String>,
    pub domain: Option<String>,
    pub port: serde_json::Value, // Port can be Int or String
    pub protocol: Option<String>,
    pub link: Option<String>,
    pub connection_string: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiUserProfile {
    pub balance: serde_json::Value, // Balance can be Float or String
    pub email: Option<String>,
}

pub async fn get_proxies(token: &str) -> Result<Vec<ApiResponseProxy>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/proxies", BASE_URL);

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if res.status() != 200 {
        return Err(format!("Server returned error code: {}", res.status()));
    }

    let proxies = res
        .json::<Vec<ApiResponseProxy>>()
        .await
        .map_err(|e| format!("Failed to parse proxies: {}", e))?;

    Ok(proxies)
}

pub async fn get_profile(token: &str) -> Result<ApiUserProfile, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/profile", BASE_URL);

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if res.status() != 200 {
        return Err(format!("Server returned error code: {}", res.status()));
    }

    let profile = res
        .json::<ApiUserProfile>()
        .await
        .map_err(|e| format!("Failed to parse profile details: {}", e))?;

    Ok(profile)
}
