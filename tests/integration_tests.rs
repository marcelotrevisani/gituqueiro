use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// We test the GitHub client by pointing it at a wiremock mock server.
// The client module isn't pub, so we replicate minimal request logic here
// using the same types.

mod helpers {
    use reqwest::Client;
    use serde::de::DeserializeOwned;

    pub struct TestClient {
        pub client: Client,
        pub base_url: String,
        pub token: String,
    }

    impl TestClient {
        pub fn new(base_url: &str) -> Self {
            Self {
                client: Client::new(),
                base_url: base_url.to_string(),
                token: "test-token".to_string(),
            }
        }

        pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
            let resp = self
                .client
                .get(format!("{}{}", self.base_url, path))
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "gituqueiro")
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }

            resp.json().await.map_err(|e| e.to_string())
        }
    }
}

#[tokio::test]
async fn test_fetch_pull_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/pulls"))
        .and(query_param("state", "open"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "number": 42,
                "title": "Fix critical bug",
                "html_url": "https://github.com/owner/repo/pull/42",
                "state": "open",
                "draft": false,
                "created_at": "2025-01-15T10:00:00Z",
                "updated_at": "2025-01-16T10:00:00Z",
                "user": { "login": "alice" },
                "head": { "sha": "abc123def", "ref": "fix-bug" },
                "labels": [
                    { "name": "bug", "color": "d73a4a" },
                    { "name": "priority", "color": "e4e669" }
                ],
                "requested_reviewers": [
                    { "login": "bob" },
                    { "login": "charlie" }
                ],
                "additions": 25,
                "deletions": 10,
                "changed_files": 3,
                "mergeable": true
            },
            {
                "number": 43,
                "title": "Add new feature",
                "html_url": "https://github.com/owner/repo/pull/43",
                "state": "open",
                "draft": true,
                "created_at": "2025-01-10T10:00:00Z",
                "updated_at": "2025-01-14T10:00:00Z",
                "user": { "login": "bob" },
                "head": { "sha": "def456", "ref": "new-feature" },
                "labels": [],
                "requested_reviewers": [],
                "additions": 500,
                "deletions": 200,
                "changed_files": 15
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let prs: Vec<gituqueiro::github::types::GhPullRequest> = client
        .get("/repos/owner/repo/pulls?state=open")
        .await
        .unwrap();

    assert_eq!(prs.len(), 2);
    assert_eq!(prs[0].number, 42);
    assert_eq!(prs[0].title, "Fix critical bug");
    assert_eq!(prs[0].user.login, "alice");
    assert_eq!(prs[0].labels.len(), 2);
    assert_eq!(prs[0].requested_reviewers.len(), 2);
    assert_eq!(prs[0].additions, Some(25));

    assert_eq!(prs[1].number, 43);
    assert_eq!(prs[1].draft, Some(true));
    assert_eq!(prs[1].user.login, "bob");
}

#[tokio::test]
async fn test_fetch_check_runs() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/commits/abc123/check-runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 3,
            "check_runs": [
                {
                    "name": "build",
                    "status": "completed",
                    "conclusion": "success",
                    "html_url": "https://github.com/owner/repo/runs/1",
                    "details_url": null
                },
                {
                    "name": "test",
                    "status": "completed",
                    "conclusion": "failure",
                    "html_url": "https://github.com/owner/repo/runs/2",
                    "details_url": null
                },
                {
                    "name": "lint",
                    "status": "in_progress",
                    "conclusion": null,
                    "html_url": "https://github.com/owner/repo/runs/3",
                    "details_url": null
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::types::GhCheckRunsResponse = client
        .get("/repos/owner/repo/commits/abc123/check-runs")
        .await
        .unwrap();

    assert_eq!(resp.total_count, 3);
    assert_eq!(resp.check_runs.len(), 3);

    use gituqueiro::github::types::CiStatus;
    let status = CiStatus::from_check_runs(&resp.check_runs);
    assert_eq!(status, CiStatus::Failure);
}

#[tokio::test]
async fn test_fetch_user() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "login": "testuser" })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let user: gituqueiro::github::types::GhUser = client.get("/user").await.unwrap();
    assert_eq!(user.login, "testuser");
}

#[tokio::test]
async fn test_unauthorized_returns_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Bad credentials"
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let result: Result<gituqueiro::github::types::GhUser, String> = client.get("/user").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("401"));
}

#[tokio::test]
async fn test_fetch_code_scanning_alerts() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/code-scanning/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "number": 1,
                "state": "open",
                "rule": {
                    "severity": "critical",
                    "description": "SQL Injection vulnerability"
                },
                "html_url": "https://github.com/owner/repo/security/code-scanning/1"
            },
            {
                "number": 2,
                "state": "open",
                "rule": {
                    "severity": "low",
                    "description": "Unused variable"
                },
                "html_url": "https://github.com/owner/repo/security/code-scanning/2"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let alerts: Vec<gituqueiro::github::types::GhCodeScanningAlert> = client
        .get("/repos/owner/repo/code-scanning/alerts")
        .await
        .unwrap();

    assert_eq!(alerts.len(), 2);
    assert_eq!(alerts[0].state, "open");
    assert_eq!(alerts[0].rule.severity, Some("critical".to_string()));
}

#[tokio::test]
async fn test_fetch_dependabot_alerts() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/dependabot/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "number": 10,
                "state": "open",
                "security_advisory": {
                    "severity": "high",
                    "summary": "Remote code execution in dependency X"
                },
                "html_url": "https://github.com/owner/repo/security/dependabot/10"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let alerts: Vec<gituqueiro::github::types::GhDependabotAlert> = client
        .get("/repos/owner/repo/dependabot/alerts")
        .await
        .unwrap();

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].number, 10);
    assert_eq!(
        alerts[0].security_advisory.as_ref().unwrap().severity,
        "high"
    );
}

#[tokio::test]
async fn test_code_scanning_404_not_enabled() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/code-scanning/alerts"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "message": "Code scanning is not enabled"
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let result: Result<Vec<gituqueiro::github::types::GhCodeScanningAlert>, String> =
        client.get("/repos/owner/repo/code-scanning/alerts").await;
    // 404 should return error in our test client (the real client handles it)
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_pr_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let prs: Vec<gituqueiro::github::types::GhPullRequest> =
        client.get("/repos/owner/repo/pulls").await.unwrap();

    assert!(prs.is_empty());
}

#[tokio::test]
async fn test_pr_conversion_and_filtering() {
    use gituqueiro::github::types::*;

    let json_prs: Vec<GhPullRequest> = serde_json::from_value(json!([
        {
            "number": 1,
            "title": "PR by alice",
            "html_url": "https://github.com/o/r/pull/1",
            "state": "open",
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "user": { "login": "alice" },
            "head": { "sha": "aaa" },
            "labels": [],
            "requested_reviewers": [{ "login": "charlie" }]
        },
        {
            "number": 2,
            "title": "PR by bob",
            "html_url": "https://github.com/o/r/pull/2",
            "state": "open",
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "user": { "login": "bob" },
            "head": { "sha": "bbb" },
            "labels": [],
            "requested_reviewers": [{ "login": "alice" }]
        }
    ]))
    .unwrap();

    let prs: Vec<PullRequest> = json_prs
        .into_iter()
        .map(|p| PullRequest::from_gh("o/r", p))
        .collect();

    // Filter: Mine (as alice)
    let mine: Vec<_> = prs.iter().filter(|p| p.author == "alice").collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].number, 1);

    // Filter: Review requested (as alice)
    let reviews: Vec<_> = prs
        .iter()
        .filter(|p| p.requested_reviewers.contains(&"alice".to_string()))
        .collect();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].number, 2);

    // Filter: By user bob
    let bobs: Vec<_> = prs.iter().filter(|p| p.author == "bob").collect();
    assert_eq!(bobs.len(), 1);
}

// ---------------------------------------------------------------------------
// OAuth Device Flow integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_oauth_device_code_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "3584d83530557fdd1f46af8289938c8ef79f9dc5",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::DeviceCodeResponse = client
        .client
        .post(format!("{}/login/device/code", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[("client_id", "test-client-id"), ("scope", "repo")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.user_code, "WDJB-MJHT");
    assert_eq!(resp.verification_uri, "https://github.com/login/device");
    assert_eq!(resp.interval, 5);
    assert_eq!(resp.expires_in, 900);
}

#[tokio::test]
async fn test_oauth_token_success_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ghu_test_token_abc123",
            "token_type": "bearer",
            "scope": "repo"
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::AccessTokenResponse = client
        .client
        .post(format!("{}/login/oauth/access_token", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[
            ("client_id", "test-id"),
            ("device_code", "test-device-code"),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.access_token, Some("ghu_test_token_abc123".to_string()));
    assert_eq!(resp.token_type, Some("bearer".to_string()));
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn test_oauth_token_pending_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending."
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::AccessTokenResponse = client
        .client
        .post(format!("{}/login/oauth/access_token", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[
            ("client_id", "test-id"),
            ("device_code", "test-device-code"),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp.access_token.is_none());
    assert_eq!(resp.error, Some("authorization_pending".to_string()));
}

#[tokio::test]
async fn test_oauth_token_slow_down_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "slow_down",
            "error_description": "Too many requests. Please wait."
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::AccessTokenResponse = client
        .client
        .post(format!("{}/login/oauth/access_token", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[("client_id", "test-id"), ("device_code", "dc")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.error, Some("slow_down".to_string()));
}

#[tokio::test]
async fn test_oauth_token_expired_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "expired_token",
            "error_description": "The device_code has expired."
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::AccessTokenResponse = client
        .client
        .post(format!("{}/login/oauth/access_token", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[("client_id", "test-id"), ("device_code", "dc")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.error, Some("expired_token".to_string()));
}

#[tokio::test]
async fn test_oauth_token_access_denied_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "access_denied",
            "error_description": "The user has denied your application access."
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let resp: gituqueiro::github::oauth::AccessTokenResponse = client
        .client
        .post(format!("{}/login/oauth/access_token", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[("client_id", "test-id"), ("device_code", "dc")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.error, Some("access_denied".to_string()));
    assert!(resp.error_description.is_some());
}

#[tokio::test]
async fn test_oauth_device_code_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Not Found",
            "documentation_url": "https://docs.github.com/"
        })))
        .mount(&mock_server)
        .await;

    let resp = helpers::TestClient::new(&mock_server.uri())
        .client
        .post(format!("{}/login/device/code", mock_server.uri()))
        .header("Accept", "application/json")
        .form(&[("client_id", "bad-id")])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
}

// ---------------------------------------------------------------------------
// Organization repos integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fetch_org_repos() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/orgs/myorg/repos"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "full_name": "myorg/repo-a", "archived": false, "disabled": false },
            { "full_name": "myorg/repo-b", "archived": false, "disabled": false },
            { "full_name": "myorg/old-repo", "archived": true, "disabled": false },
            { "full_name": "myorg/disabled-repo", "archived": false, "disabled": true }
        ])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let repos: Vec<gituqueiro::github::types::GhRepo> = client
        .get("/orgs/myorg/repos?type=all&per_page=100&page=1")
        .await
        .unwrap();

    assert_eq!(repos.len(), 4);

    // Filter out archived and disabled (as the client does)
    let active: Vec<_> = repos
        .iter()
        .filter(|r| !r.archived.unwrap_or(false) && !r.disabled.unwrap_or(false))
        .collect();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].full_name, "myorg/repo-a");
    assert_eq!(active[1].full_name, "myorg/repo-b");
}

#[tokio::test]
async fn test_fetch_org_repos_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/orgs/empty-org/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let repos: Vec<gituqueiro::github::types::GhRepo> = client
        .get("/orgs/empty-org/repos?type=all&per_page=100&page=1")
        .await
        .unwrap();

    assert!(repos.is_empty());
}

#[tokio::test]
async fn test_fetch_org_repos_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/orgs/nonexistent/repos"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "message": "Not Found"
        })))
        .mount(&mock_server)
        .await;

    let client = helpers::TestClient::new(&mock_server.uri());
    let result: Result<Vec<gituqueiro::github::types::GhRepo>, String> =
        client.get("/orgs/nonexistent/repos").await;

    assert!(result.is_err());
}
