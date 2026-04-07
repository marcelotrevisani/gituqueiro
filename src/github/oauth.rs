use crate::github::build_http_client;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// To use OAuth login, register a GitHub OAuth App:
//   1. Go to GitHub > Settings > Developer settings > OAuth Apps > New OAuth App
//   2. Set "Authorization callback URL" to http://localhost (not used in device flow)
//   3. After creation, enable "Device Flow" in the app settings
//   4. Replace this constant with your app's Client ID
// ---------------------------------------------------------------------------
pub const GITHUB_OAUTH_CLIENT_ID: &str = "REPLACE_WITH_YOUR_CLIENT_ID";

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

// Scopes needed for PR monitoring + security alerts
const SCOPES: &str = "repo read:org";

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

// ---------------------------------------------------------------------------
// Device Flow functions
// ---------------------------------------------------------------------------

/// Step 1: Request a device code and user code from GitHub.
/// Returns the codes the user needs to enter at verification_uri.
pub async fn start_device_flow(client_id: &str) -> Result<DeviceCodeResponse, String> {
    let client = build_http_client();
    let resp = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", SCOPES)])
        .send()
        .await
        .map_err(|e| format!("Failed to start device flow: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GitHub returned {status}: {body}"));
    }

    resp.json()
        .await
        .map_err(|e| format!("Failed to parse device code response: {e}"))
}

/// Step 2: Poll GitHub until the user authorises the app (or codes expire).
/// This function blocks (async) until a token is received or an error occurs.
pub async fn poll_for_token(
    client_id: String,
    device_code: String,
    interval: u64,
) -> Result<String, String> {
    let client = build_http_client();
    let mut interval_secs = interval;
    let grant_type = "urn:ietf:params:oauth:grant-type:device_code".to_string();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        let resp = client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", device_code.as_str()),
                ("grant_type", grant_type.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("Poll request failed: {e}"))?;

        let token_resp: AccessTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {e}"))?;

        if let Some(token) = token_resp.access_token {
            if !token.is_empty() {
                return Ok(token);
            }
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval_secs += 5;
                continue;
            }
            Some("expired_token") => {
                return Err("Device code expired. Please try again.".to_string());
            }
            Some("access_denied") => {
                return Err("Access denied by user.".to_string());
            }
            Some(err) => {
                let desc = token_resp
                    .error_description
                    .unwrap_or_else(|| "no details".to_string());
                return Err(format!("OAuth error ({err}): {desc}"));
            }
            None => {
                return Err("Unexpected OAuth response with no token and no error".to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_device_code_response() {
        let json = r#"{
            "device_code": "3584d83530557fdd1f46af8289938c8ef79f9dc5",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;

        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_code, "WDJB-MJHT");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.expires_in, 900);
        assert_eq!(resp.interval, 5);
        assert_eq!(resp.device_code, "3584d83530557fdd1f46af8289938c8ef79f9dc5");
    }

    #[test]
    fn test_deserialize_access_token_success() {
        let json = r#"{
            "access_token": "ghu_16C7e42F292c6912E7710c838347Ae178B4a",
            "token_type": "bearer",
            "scope": "repo,gist"
        }"#;

        let resp: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            resp.access_token,
            Some("ghu_16C7e42F292c6912E7710c838347Ae178B4a".to_string())
        );
        assert_eq!(resp.token_type, Some("bearer".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_deserialize_access_token_pending() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending."
        }"#;

        let resp: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert!(resp.access_token.is_none());
        assert_eq!(resp.error, Some("authorization_pending".to_string()));
        assert!(resp.error_description.is_some());
    }

    #[test]
    fn test_deserialize_access_token_slow_down() {
        let json = r#"{
            "error": "slow_down",
            "error_description": "Too many requests."
        }"#;

        let resp: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, Some("slow_down".to_string()));
    }

    #[test]
    fn test_deserialize_access_token_expired() {
        let json = r#"{
            "error": "expired_token",
            "error_description": "The device_code has expired."
        }"#;

        let resp: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, Some("expired_token".to_string()));
    }

    #[test]
    fn test_deserialize_access_token_denied() {
        let json = r#"{
            "error": "access_denied",
            "error_description": "The user has denied your application access."
        }"#;

        let resp: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, Some("access_denied".to_string()));
    }

    #[test]
    fn test_constants() {
        assert!(!SCOPES.is_empty());
        assert!(DEVICE_CODE_URL.starts_with("https://"));
        assert!(ACCESS_TOKEN_URL.starts_with("https://"));
    }
}
