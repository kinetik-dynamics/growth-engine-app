//! HTTP client for the Growth Engine web app (Sanctum personal access token).

use serde_json::Value;
use std::time::Duration;

/// Trims the server URL and makes sure it is an http(s) origin.
pub fn normalize_base_url(input: &str) -> Result<String, String> {
    let url = input.trim().trim_end_matches('/');
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Server URL must start with http:// or https://".into());
    }
    Ok(url.to_string())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn get(base_url: &str, token: &str, path: &str) -> Result<Value, String> {
    get_query(base_url, token, path, &[]).await
}

/// A GET with query parameters, which are URL-encoded for the caller.
pub async fn get_query(base_url: &str, token: &str, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
    let request = client()?.get(format!("{base_url}{path}")).query(query);
    send(request, base_url, token, path).await
}

pub async fn post(base_url: &str, token: &str, path: &str, body: &Value) -> Result<Value, String> {
    let request = client()?.post(format!("{base_url}{path}")).json(body);
    send(request, base_url, token, path).await
}

async fn send(request: reqwest::RequestBuilder, base_url: &str, token: &str, path: &str) -> Result<Value, String> {
    let response = request
        .bearer_auth(token.trim())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Couldn't reach {base_url}. Check the URL and that the site is running. ({e})"))?;

    let status = response.status();
    if status.as_u16() == 401 {
        return Err("The server rejected this token. Create a new one under Settings > Access Tokens and try again.".into());
    }
    if status.as_u16() == 422 {
        // Laravel validation errors carry a readable message.
        let body = response.json::<Value>().await.unwrap_or_default();
        let message = body["message"].as_str().unwrap_or("The server rejected the data.");
        return Err(format!("The main site didn't accept this: {message}"));
    }
    if !status.is_success() {
        return Err(format!("The server answered {status} for {path}."));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("The server sent an unexpected response for {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

    #[test]
    fn base_url_is_trimmed_and_requires_a_scheme() {
        assert_eq!(
            normalize_base_url(" http://growth-engine.test/ ").unwrap(),
            "http://growth-engine.test"
        );
        assert!(normalize_base_url("growth-engine.test").is_err());
    }
}
