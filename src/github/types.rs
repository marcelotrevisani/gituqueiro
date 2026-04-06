use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt;

// ---------------------------------------------------------------------------
// GitHub API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GhPullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    pub draft: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user: GhUser,
    pub head: GhRef,
    pub labels: Vec<GhLabel>,
    #[serde(default)]
    pub requested_reviewers: Vec<GhUser>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub mergeable: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhUser {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRef {
    pub sha: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhLabel {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhCheckRunsResponse {
    pub total_count: u64,
    pub check_runs: Vec<GhCheckRun>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhCheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
    pub details_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRepo {
    pub full_name: String,
    pub archived: Option<bool>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhCodeScanningAlert {
    pub number: u64,
    pub state: String,
    pub rule: GhRule,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRule {
    pub severity: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhDependabotAlert {
    pub number: u64,
    pub state: String,
    pub security_advisory: Option<GhSecurityAdvisory>,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhSecurityAdvisory {
    pub severity: String,
    pub summary: String,
}

// ---------------------------------------------------------------------------
// Application-level types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub repo_full_name: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub html_url: String,
    pub state: String,
    pub draft: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub labels: Vec<(String, String)>,
    pub requested_reviewers: Vec<String>,
    pub head_sha: String,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
    pub ci_status: CiStatus,
    pub ci_url: Option<String>,
    pub checks: Vec<CheckInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CiStatus {
    Unknown,
    Pending,
    Running,
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct CheckInfo {
    pub name: String,
    pub status: CiStatus,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct RepoHealth {
    pub repo_full_name: String,
    pub code_scanning_alerts: Vec<SecurityAlert>,
    pub dependabot_alerts: Vec<DependabotAlert>,
    pub open_prs_count: u64,
    pub stale_prs_count: u64,
}

#[derive(Debug, Clone)]
pub struct SecurityAlert {
    pub number: u64,
    pub severity: String,
    pub description: String,
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct DependabotAlert {
    pub number: u64,
    pub severity: String,
    pub summary: String,
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrFilter {
    All,
    Mine,
    ReviewRequested,
    ByUser(String),
}

impl PrFilter {
    pub const CHOICES: &[&str] = &["All PRs", "My PRs", "Review Requested", "By User"];
}

impl fmt::Display for PrFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrFilter::All => write!(f, "All PRs"),
            PrFilter::Mine => write!(f, "My PRs"),
            PrFilter::ReviewRequested => write!(f, "Review Requested"),
            PrFilter::ByUser(u) => write!(f, "By User: {u}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Repo,
    Number,
    Title,
    Author,
    CiStatus,
    Age,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    PullRequests,
    RepoHealth,
    Settings,
}

// ---------------------------------------------------------------------------
// Conversions & helpers
// ---------------------------------------------------------------------------

impl PullRequest {
    pub fn from_gh(repo: &str, gh: GhPullRequest) -> Self {
        Self {
            repo_full_name: repo.to_string(),
            number: gh.number,
            title: gh.title,
            author: gh.user.login,
            html_url: gh.html_url,
            state: gh.state,
            draft: gh.draft.unwrap_or(false),
            created_at: gh.created_at,
            updated_at: gh.updated_at,
            labels: gh.labels.into_iter().map(|l| (l.name, l.color)).collect(),
            requested_reviewers: gh
                .requested_reviewers
                .into_iter()
                .map(|u| u.login)
                .collect(),
            head_sha: gh.head.sha,
            additions: gh.additions.unwrap_or(0),
            deletions: gh.deletions.unwrap_or(0),
            changed_files: gh.changed_files.unwrap_or(0),
            ci_status: CiStatus::Unknown,
            ci_url: None,
            checks: vec![],
        }
    }

    pub fn age_days(&self) -> i64 {
        (Utc::now() - self.created_at).num_days()
    }

    pub fn age_display(&self) -> String {
        let days = self.age_days();
        if days == 0 {
            let hours = (Utc::now() - self.created_at).num_hours();
            if hours == 0 {
                let mins = (Utc::now() - self.created_at).num_minutes();
                format!("{mins}m")
            } else {
                format!("{hours}h")
            }
        } else {
            format!("{days}d")
        }
    }

    pub fn size_label(&self) -> &str {
        let total = self.additions + self.deletions;
        match total {
            0..=10 => "XS",
            11..=50 => "S",
            51..=200 => "M",
            201..=500 => "L",
            _ => "XL",
        }
    }

    pub fn is_stale(&self) -> bool {
        self.age_days() > 30
    }
}

impl CiStatus {
    pub fn from_check_runs(runs: &[GhCheckRun]) -> Self {
        if runs.is_empty() {
            return CiStatus::Unknown;
        }

        let mut has_failure = false;
        let mut has_pending = false;
        let mut has_running = false;

        for run in runs {
            match run.status.as_str() {
                "queued" | "waiting" => has_pending = true,
                "in_progress" => has_running = true,
                "completed" => match run.conclusion.as_deref() {
                    Some("success") | Some("skipped") | Some("neutral") => {}
                    Some("failure")
                    | Some("timed_out")
                    | Some("action_required")
                    | Some("cancelled") => has_failure = true,
                    _ => {}
                },
                _ => {}
            }
        }

        if has_failure {
            CiStatus::Failure
        } else if has_running {
            CiStatus::Running
        } else if has_pending {
            CiStatus::Pending
        } else {
            CiStatus::Success
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            CiStatus::Unknown => "?",
            CiStatus::Pending => "~",
            CiStatus::Running => "~",
            CiStatus::Success => "OK",
            CiStatus::Failure => "X",
        }
    }

    pub fn sort_order(&self) -> u8 {
        match self {
            CiStatus::Failure => 0,
            CiStatus::Running => 1,
            CiStatus::Pending => 2,
            CiStatus::Unknown => 3,
            CiStatus::Success => 4,
        }
    }
}

impl RepoHealth {
    pub fn critical_count(&self) -> usize {
        self.code_scanning_alerts
            .iter()
            .filter(|a| a.severity == "critical" || a.severity == "high")
            .count()
            + self
                .dependabot_alerts
                .iter()
                .filter(|a| a.severity == "critical" || a.severity == "high")
                .count()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gh_pr() -> GhPullRequest {
        GhPullRequest {
            number: 42,
            title: "Fix the thing".to_string(),
            html_url: "https://github.com/owner/repo/pull/42".to_string(),
            state: "open".to_string(),
            draft: Some(false),
            created_at: Utc::now() - chrono::Duration::days(3),
            updated_at: Utc::now(),
            user: GhUser {
                login: "alice".to_string(),
            },
            head: GhRef {
                sha: "abc123".to_string(),
                ref_name: Some("fix-branch".to_string()),
            },
            labels: vec![GhLabel {
                name: "bug".to_string(),
                color: "d73a4a".to_string(),
            }],
            requested_reviewers: vec![GhUser {
                login: "bob".to_string(),
            }],
            additions: Some(10),
            deletions: Some(5),
            changed_files: Some(2),
            mergeable: Some(true),
        }
    }

    #[test]
    fn test_pr_from_gh() {
        let gh = sample_gh_pr();
        let pr = PullRequest::from_gh("owner/repo", gh);

        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix the thing");
        assert_eq!(pr.author, "alice");
        assert_eq!(pr.repo_full_name, "owner/repo");
        assert!(!pr.draft);
        assert_eq!(pr.labels.len(), 1);
        assert_eq!(pr.labels[0].0, "bug");
        assert_eq!(pr.requested_reviewers, vec!["bob".to_string()]);
        assert_eq!(pr.additions, 10);
        assert_eq!(pr.deletions, 5);
        assert_eq!(pr.head_sha, "abc123");
        assert_eq!(pr.ci_status, CiStatus::Unknown);
    }

    #[test]
    fn test_pr_age_days() {
        let mut gh = sample_gh_pr();
        gh.created_at = Utc::now() - chrono::Duration::days(5);
        let pr = PullRequest::from_gh("owner/repo", gh);
        assert_eq!(pr.age_days(), 5);
    }

    #[test]
    fn test_pr_size_label() {
        let mut gh = sample_gh_pr();

        gh.additions = Some(3);
        gh.deletions = Some(2);
        let pr = PullRequest::from_gh("r", gh.clone());
        assert_eq!(pr.size_label(), "XS");

        gh.additions = Some(30);
        gh.deletions = Some(10);
        let pr = PullRequest::from_gh("r", gh.clone());
        assert_eq!(pr.size_label(), "S");

        gh.additions = Some(100);
        gh.deletions = Some(50);
        let pr = PullRequest::from_gh("r", gh.clone());
        assert_eq!(pr.size_label(), "M");

        gh.additions = Some(300);
        gh.deletions = Some(100);
        let pr = PullRequest::from_gh("r", gh.clone());
        assert_eq!(pr.size_label(), "L");

        gh.additions = Some(1000);
        gh.deletions = Some(500);
        let pr = PullRequest::from_gh("r", gh);
        assert_eq!(pr.size_label(), "XL");
    }

    #[test]
    fn test_pr_is_stale() {
        let mut gh = sample_gh_pr();

        gh.created_at = Utc::now() - chrono::Duration::days(31);
        let pr = PullRequest::from_gh("r", gh.clone());
        assert!(pr.is_stale());

        gh.created_at = Utc::now() - chrono::Duration::days(10);
        let pr = PullRequest::from_gh("r", gh);
        assert!(!pr.is_stale());
    }

    #[test]
    fn test_ci_status_all_success() {
        let runs = vec![
            GhCheckRun {
                name: "build".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                html_url: "".to_string(),
                details_url: None,
            },
            GhCheckRun {
                name: "test".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                html_url: "".to_string(),
                details_url: None,
            },
        ];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Success);
    }

    #[test]
    fn test_ci_status_with_failure() {
        let runs = vec![
            GhCheckRun {
                name: "build".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                html_url: "".to_string(),
                details_url: None,
            },
            GhCheckRun {
                name: "test".to_string(),
                status: "completed".to_string(),
                conclusion: Some("failure".to_string()),
                html_url: "".to_string(),
                details_url: None,
            },
        ];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Failure);
    }

    #[test]
    fn test_ci_status_running() {
        let runs = vec![GhCheckRun {
            name: "build".to_string(),
            status: "in_progress".to_string(),
            conclusion: None,
            html_url: "".to_string(),
            details_url: None,
        }];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Running);
    }

    #[test]
    fn test_ci_status_pending() {
        let runs = vec![GhCheckRun {
            name: "build".to_string(),
            status: "queued".to_string(),
            conclusion: None,
            html_url: "".to_string(),
            details_url: None,
        }];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Pending);
    }

    #[test]
    fn test_ci_status_empty() {
        assert_eq!(CiStatus::from_check_runs(&[]), CiStatus::Unknown);
    }

    #[test]
    fn test_ci_status_skipped_counts_as_success() {
        let runs = vec![GhCheckRun {
            name: "optional".to_string(),
            status: "completed".to_string(),
            conclusion: Some("skipped".to_string()),
            html_url: "".to_string(),
            details_url: None,
        }];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Success);
    }

    #[test]
    fn test_ci_status_failure_takes_priority() {
        let runs = vec![
            GhCheckRun {
                name: "build".to_string(),
                status: "in_progress".to_string(),
                conclusion: None,
                html_url: "".to_string(),
                details_url: None,
            },
            GhCheckRun {
                name: "test".to_string(),
                status: "completed".to_string(),
                conclusion: Some("failure".to_string()),
                html_url: "".to_string(),
                details_url: None,
            },
        ];
        assert_eq!(CiStatus::from_check_runs(&runs), CiStatus::Failure);
    }

    #[test]
    fn test_pr_filter_display() {
        assert_eq!(PrFilter::All.to_string(), "All PRs");
        assert_eq!(PrFilter::Mine.to_string(), "My PRs");
        assert_eq!(PrFilter::ReviewRequested.to_string(), "Review Requested");
        assert_eq!(
            PrFilter::ByUser("alice".to_string()).to_string(),
            "By User: alice"
        );
    }

    #[test]
    fn test_ci_status_sort_order() {
        assert!(CiStatus::Failure.sort_order() < CiStatus::Running.sort_order());
        assert!(CiStatus::Running.sort_order() < CiStatus::Success.sort_order());
    }

    #[test]
    fn test_repo_health_critical_count() {
        let health = RepoHealth {
            repo_full_name: "owner/repo".to_string(),
            code_scanning_alerts: vec![
                SecurityAlert {
                    number: 1,
                    severity: "critical".to_string(),
                    description: "Bad".to_string(),
                    url: "".to_string(),
                    state: "open".to_string(),
                },
                SecurityAlert {
                    number: 2,
                    severity: "low".to_string(),
                    description: "Minor".to_string(),
                    url: "".to_string(),
                    state: "open".to_string(),
                },
            ],
            dependabot_alerts: vec![DependabotAlert {
                number: 1,
                severity: "high".to_string(),
                summary: "Vuln".to_string(),
                url: "".to_string(),
                state: "open".to_string(),
            }],
            open_prs_count: 5,
            stale_prs_count: 1,
        };
        assert_eq!(health.critical_count(), 2);
    }

    #[test]
    fn test_deserialize_gh_pull_request() {
        let json = r#"{
            "number": 1,
            "title": "Test PR",
            "html_url": "https://github.com/o/r/pull/1",
            "state": "open",
            "draft": false,
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-02T00:00:00Z",
            "user": { "login": "alice" },
            "head": { "sha": "abc123", "ref": "feature" },
            "labels": [{ "name": "bug", "color": "d73a4a" }],
            "requested_reviewers": [{ "login": "bob" }],
            "additions": 10,
            "deletions": 5,
            "changed_files": 3,
            "mergeable": true
        }"#;

        let pr: GhPullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 1);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.user.login, "alice");
        assert_eq!(pr.head.sha, "abc123");
        assert_eq!(pr.head.ref_name, Some("feature".to_string()));
        assert_eq!(pr.labels[0].name, "bug");
    }

    #[test]
    fn test_deserialize_gh_check_runs_response() {
        let json = r#"{
            "total_count": 1,
            "check_runs": [{
                "name": "CI",
                "status": "completed",
                "conclusion": "success",
                "html_url": "https://github.com/o/r/runs/1",
                "details_url": null
            }]
        }"#;

        let resp: GhCheckRunsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.check_runs[0].name, "CI");
        assert_eq!(resp.check_runs[0].conclusion, Some("success".to_string()));
    }

    #[test]
    fn test_deserialize_minimal_pr() {
        let json = r#"{
            "number": 1,
            "title": "Minimal",
            "html_url": "https://github.com/o/r/pull/1",
            "state": "open",
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "user": { "login": "alice" },
            "head": { "sha": "abc" },
            "labels": []
        }"#;

        let pr: GhPullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 1);
        assert!(pr.draft.is_none());
        assert!(pr.additions.is_none());
        assert!(pr.requested_reviewers.is_empty());
    }

    #[test]
    fn test_deserialize_gh_repo() {
        let json = r#"{
            "full_name": "myorg/myrepo",
            "archived": false,
            "disabled": false
        }"#;
        let repo: GhRepo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.full_name, "myorg/myrepo");
        assert_eq!(repo.archived, Some(false));
    }

    #[test]
    fn test_deserialize_gh_repo_minimal() {
        let json = r#"{ "full_name": "org/repo" }"#;
        let repo: GhRepo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.full_name, "org/repo");
        assert!(repo.archived.is_none());
    }
}
