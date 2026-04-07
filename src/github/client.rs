use crate::github::types::*;
use crate::github::build_http_client;
use reqwest::Client;

pub struct GitHubClient {
    client: Client,
    token: String,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: &str) -> Self {
        Self {
            client: build_http_client(),
            token: token.to_string(),
            base_url: "https://api.github.com".to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(token: &str, base_url: &str) -> Self {
        Self {
            client: build_http_client(),
            token: token.to_string(),
            base_url: base_url.to_string(),
        }
    }

    fn auth_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gituqueiro")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    pub async fn get_authenticated_user(&self) -> Result<String, String> {
        let req = self.client.get(format!("{}/user", self.base_url));
        let resp = self
            .auth_headers(req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Auth failed: {}", resp.status()));
        }

        let user: GhUser = resp.json().await.map_err(|e| e.to_string())?;
        Ok(user.login)
    }

    pub async fn fetch_pull_requests(&self, repo: &str) -> Result<Vec<PullRequest>, String> {
        let req = self
            .client
            .get(format!("{}/repos/{}/pulls", self.base_url, repo))
            .query(&[("state", "open"), ("per_page", "100")]);
        let resp = self
            .auth_headers(req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch PRs for {}: {}",
                repo,
                resp.status()
            ));
        }

        let prs: Vec<GhPullRequest> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(prs
            .into_iter()
            .map(|pr| PullRequest::from_gh(repo, pr))
            .collect())
    }

    pub async fn fetch_check_runs(
        &self,
        repo: &str,
        sha: &str,
    ) -> Result<GhCheckRunsResponse, String> {
        let req = self.client.get(format!(
            "{}/repos/{}/commits/{}/check-runs",
            self.base_url, repo, sha
        ));
        let resp = self
            .auth_headers(req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch checks for {}: {}",
                repo,
                resp.status()
            ));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn fetch_code_scanning_alerts(
        &self,
        repo: &str,
    ) -> Result<Vec<GhCodeScanningAlert>, String> {
        let req = self
            .client
            .get(format!(
                "{}/repos/{}/code-scanning/alerts",
                self.base_url, repo
            ))
            .query(&[("state", "open"), ("per_page", "100")]);
        let resp = self
            .auth_headers(req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().as_u16() == 404 || resp.status().as_u16() == 403 {
            return Ok(vec![]);
        }
        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch code scanning for {}: {}",
                repo,
                resp.status()
            ));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Fetch all non-archived repos for an organization (paginated).
    pub async fn fetch_org_repos(&self, org: &str) -> Result<Vec<String>, String> {
        let mut repos = Vec::new();
        let mut page = 1u32;

        loop {
            let req = self
                .client
                .get(format!("{}/orgs/{}/repos", self.base_url, org))
                .query(&[
                    ("type", "all"),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ]);
            let resp = self
                .auth_headers(req)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                return Err(format!(
                    "Failed to list repos for org {}: {}",
                    org,
                    resp.status()
                ));
            }

            let batch: Vec<GhRepo> = resp.json().await.map_err(|e| e.to_string())?;
            if batch.is_empty() {
                break;
            }

            for r in &batch {
                let archived = r.archived.unwrap_or(false);
                let disabled = r.disabled.unwrap_or(false);
                if !archived && !disabled {
                    repos.push(r.full_name.clone());
                }
            }

            if batch.len() < 100 {
                break;
            }
            page += 1;
        }

        Ok(repos)
    }

    pub async fn fetch_dependabot_alerts(
        &self,
        repo: &str,
    ) -> Result<Vec<GhDependabotAlert>, String> {
        let req = self
            .client
            .get(format!(
                "{}/repos/{}/dependabot/alerts",
                self.base_url, repo
            ))
            .query(&[("state", "open"), ("per_page", "100")]);
        let resp = self
            .auth_headers(req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().as_u16() == 404 || resp.status().as_u16() == 403 {
            return Ok(vec![]);
        }
        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch dependabot alerts for {}: {}",
                repo,
                resp.status()
            ));
        }

        resp.json().await.map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Top-level fetch functions used by the iced app
// ---------------------------------------------------------------------------

/// Expand a list of repos + orgs into a flat list of repo full names.
pub async fn resolve_repos(
    token: String,
    repos: Vec<String>,
    orgs: Vec<String>,
) -> Result<Vec<String>, String> {
    let client = GitHubClient::new(&token);
    let mut all_repos: Vec<String> = repos;

    for org in &orgs {
        let org_repos = client.fetch_org_repos(org).await?;
        for r in org_repos {
            if !all_repos.contains(&r) {
                all_repos.push(r);
            }
        }
    }

    Ok(all_repos)
}

/// Fetch PRs + CI status for a single repo, applying the given filter.
pub async fn fetch_repo_prs(
    token: String,
    repo: String,
    filter: PrFilter,
    username: Option<String>,
) -> (String, Result<Vec<PullRequest>, String>) {
    let result = fetch_repo_prs_inner(&token, &repo, &filter, username.as_deref()).await;
    (repo, result)
}

async fn fetch_repo_prs_inner(
    token: &str,
    repo: &str,
    filter: &PrFilter,
    username: Option<&str>,
) -> Result<Vec<PullRequest>, String> {
    let client = GitHubClient::new(token);
    let mut prs = client.fetch_pull_requests(repo).await?;

    match filter {
        PrFilter::All => {}
        PrFilter::Mine => {
            if let Some(user) = username {
                prs.retain(|pr| pr.author == user);
            }
        }
        PrFilter::ReviewRequested => {
            if let Some(user) = username {
                prs.retain(|pr| pr.requested_reviewers.contains(&user.to_string()));
            }
        }
        PrFilter::ByUser(target) => {
            prs.retain(|pr| pr.author == *target);
        }
    }

    for pr in &mut prs {
        if let Ok(checks) = client.fetch_check_runs(repo, &pr.head_sha).await {
            pr.ci_status = CiStatus::from_check_runs(&checks.check_runs);
            pr.checks = checks
                .check_runs
                .iter()
                .map(|c| CheckInfo {
                    name: c.name.clone(),
                    status: CiStatus::from_check_runs(std::slice::from_ref(c)),
                    url: c.html_url.clone(),
                })
                .collect();
            if !checks.check_runs.is_empty() {
                pr.ci_url = Some(format!(
                    "https://github.com/{}/pull/{}/checks",
                    repo, pr.number
                ));
            }
        }
    }

    Ok(prs)
}

pub async fn fetch_repo_health(
    token: String,
    repos: Vec<String>,
    orgs: Vec<String>,
) -> Result<Vec<RepoHealth>, String> {
    let effective_repos = resolve_repos(token.clone(), repos, orgs).await?;
    let client = GitHubClient::new(&token);
    let mut health_reports = Vec::new();

    for repo in &effective_repos {
        let code_alerts = client
            .fetch_code_scanning_alerts(repo)
            .await
            .unwrap_or_default();
        let dep_alerts = client
            .fetch_dependabot_alerts(repo)
            .await
            .unwrap_or_default();
        let prs = client.fetch_pull_requests(repo).await.unwrap_or_default();

        let stale_count = prs.iter().filter(|pr| pr.is_stale()).count() as u64;

        health_reports.push(RepoHealth {
            repo_full_name: repo.clone(),
            code_scanning_alerts: code_alerts
                .into_iter()
                .map(|a| SecurityAlert {
                    number: a.number,
                    severity: a.rule.severity.unwrap_or_else(|| "unknown".to_string()),
                    description: a.rule.description.unwrap_or_default(),
                    url: a.html_url,
                    state: a.state,
                })
                .collect(),
            dependabot_alerts: dep_alerts
                .into_iter()
                .map(|a| DependabotAlert {
                    number: a.number,
                    severity: a
                        .security_advisory
                        .as_ref()
                        .map(|s| s.severity.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    summary: a
                        .security_advisory
                        .as_ref()
                        .map(|s| s.summary.clone())
                        .unwrap_or_default(),
                    url: a.html_url,
                    state: a.state,
                })
                .collect(),
            open_prs_count: prs.len() as u64,
            stale_prs_count: stale_count,
        });
    }

    Ok(health_reports)
}

pub async fn fetch_username(token: String) -> Result<String, String> {
    let client = GitHubClient::new(&token);
    client.get_authenticated_user().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_mine() {
        let prs = vec![
            make_test_pr("alice", &[]),
            make_test_pr("bob", &[]),
            make_test_pr("alice", &[]),
        ];

        let filtered: Vec<_> = prs.into_iter().filter(|pr| pr.author == "alice").collect();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_review_requested() {
        let prs = vec![
            make_test_pr("alice", &["charlie"]),
            make_test_pr("bob", &["charlie", "dave"]),
            make_test_pr("alice", &["dave"]),
        ];

        let user = "charlie".to_string();
        let filtered: Vec<_> = prs
            .into_iter()
            .filter(|pr| pr.requested_reviewers.contains(&user))
            .collect();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_user() {
        let prs = vec![make_test_pr("alice", &[]), make_test_pr("bob", &[])];

        let target = "bob";
        let filtered: Vec<_> = prs.into_iter().filter(|pr| pr.author == target).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].author, "bob");
    }

    fn make_test_pr(author: &str, reviewers: &[&str]) -> PullRequest {
        PullRequest {
            repo_full_name: "owner/repo".to_string(),
            number: 1,
            title: "Test".to_string(),
            author: author.to_string(),
            html_url: "https://example.com".to_string(),
            state: "open".to_string(),
            draft: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            labels: vec![],
            requested_reviewers: reviewers.iter().map(|s| s.to_string()).collect(),
            head_sha: "abc".to_string(),
            additions: 0,
            deletions: 0,
            changed_files: 0,
            ci_status: CiStatus::Unknown,
            ci_url: None,
            checks: vec![],
        }
    }
}
