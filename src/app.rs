use crate::config::Config;
use crate::github::client;
use crate::github::oauth;
use crate::github::types::*;
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, row, scrollable, text, text_input,
    tooltip, Space,
};
use iced::{color, Element, Length, Subscription, Task, Theme};
use std::time::Instant;

// ---------------------------------------------------------------------------
// OAuth state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum OAuthStatus {
    Idle,
    WaitingForUser,
    Polling,
    Error(String),
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct Gituqueiro {
    config: Config,
    token_input: String,
    repo_input: String,
    user_filter_input: String,
    search_input: String,
    oauth_client_id_input: String,
    org_input: String,

    pull_requests: Vec<PullRequest>,
    repo_health: Vec<RepoHealth>,
    loading: bool,
    error: Option<String>,
    status_message: Option<String>,

    active_tab: Tab,
    filter: PrFilter,
    sort_by: SortField,
    sort_ascending: bool,
    auto_refresh: bool,
    show_token: bool,

    last_clicked_pr: Option<usize>,
    last_click_time: Option<Instant>,

    selected_filter_label: String,

    // OAuth Device Flow state
    oauth_status: OAuthStatus,
    oauth_user_code: Option<String>,
    oauth_verification_uri: Option<String>,

    // Incremental loading progress
    loading_repos_total: usize,
    loading_repos_done: usize,

    // Double-click tracking for connection status
    last_conn_click: Option<Instant>,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // Repo management
    RepoInputChanged(String),
    AddRepo,
    RemoveRepo(usize),

    // Org management
    OrgInputChanged(String),
    AddOrg,
    RemoveOrg(usize),

    // Filters
    UserFilterChanged(String),
    SearchChanged(String),
    SetFilterAll,
    SetFilterMine,
    SetFilterReview,
    SetFilterByUser,

    // Sort
    SortBy(SortField),

    // Actions
    Refresh,
    ToggleAutoRefresh(bool),
    ClickPr(usize),
    OpenUrl(String),

    // Tab
    SetTab(Tab),

    // Settings inputs
    TokenChanged(String),
    ToggleShowToken,
    OAuthClientIdChanged(String),
    SaveSettings,

    // Connection status double-click
    ClickConnectionStatus,

    // OAuth Device Flow
    StartOAuth,
    DeviceCodeReceived(Result<oauth::DeviceCodeResponse, String>),
    OAuthTokenReceived(Result<String, String>),
    CancelOAuth,

    // Async results (incremental)
    ReposResolved(Result<Vec<String>, String>),
    RepoPrsLoaded(String, Result<Vec<PullRequest>, String>),
    RepoHealthLoaded(Result<Vec<RepoHealth>, String>),
    UsernameLoaded(Result<String, String>),
    SavedConfig(Result<(), String>),

    // Auto-refresh tick
    Tick(Instant),
}

// ---------------------------------------------------------------------------
// Application logic
// ---------------------------------------------------------------------------

impl Gituqueiro {
    pub fn new() -> (Self, Task<Message>) {
        let config = Config::load();
        let token_input = config.token.clone();
        let auto_refresh = config.auto_refresh_seconds > 0;
        let oauth_client_id_input = config.oauth_client_id.clone().unwrap_or_default();

        let mut app = Self {
            config,
            token_input,
            repo_input: String::new(),
            org_input: String::new(),
            user_filter_input: String::new(),
            search_input: String::new(),
            oauth_client_id_input,
            pull_requests: Vec::new(),
            repo_health: Vec::new(),
            loading: false,
            error: None,
            status_message: None,
            active_tab: Tab::PullRequests,
            filter: PrFilter::All,
            sort_by: SortField::Age,
            sort_ascending: false,
            auto_refresh,
            show_token: false,
            last_clicked_pr: None,
            last_click_time: None,
            selected_filter_label: "All PRs".to_string(),
            oauth_status: OAuthStatus::Idle,
            oauth_user_code: None,
            oauth_verification_uri: None,
            loading_repos_total: 0,
            loading_repos_done: 0,
            last_conn_click: None,
        };

        let task = if !app.config.token.is_empty() {
            app.loading = true;
            let token = app.config.token.clone();
            Task::perform(client::fetch_username(token), Message::UsernameLoaded)
        } else {
            Task::none()
        };

        (app, task)
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.auto_refresh && !self.config.token.is_empty() {
            let secs = if self.config.auto_refresh_seconds > 0 {
                self.config.auto_refresh_seconds
            } else {
                300
            };
            iced::time::every(std::time::Duration::from_secs(secs)).map(Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // --- Repo management ---
            Message::RepoInputChanged(val) => {
                self.repo_input = val;
                Task::none()
            }
            Message::AddRepo => {
                let repo = self.repo_input.trim().to_string();
                if !repo.is_empty() && repo.contains('/') && !self.config.repos.contains(&repo) {
                    self.config.repos.push(repo);
                    self.repo_input.clear();
                    return self.save_config();
                }
                Task::none()
            }
            Message::RemoveRepo(idx) => {
                if idx < self.config.repos.len() {
                    self.config.repos.remove(idx);
                    return self.save_config();
                }
                Task::none()
            }
            // --- Org management ---
            Message::OrgInputChanged(val) => {
                self.org_input = val;
                Task::none()
            }
            Message::AddOrg => {
                let org = self.org_input.trim().to_string();
                if !org.is_empty() && !org.contains('/') && !self.config.orgs.contains(&org) {
                    self.config.orgs.push(org);
                    self.org_input.clear();
                    return self.save_config();
                }
                Task::none()
            }
            Message::RemoveOrg(idx) => {
                if idx < self.config.orgs.len() {
                    self.config.orgs.remove(idx);
                    return self.save_config();
                }
                Task::none()
            }
            // --- Filters ---
            Message::UserFilterChanged(val) => {
                self.user_filter_input = val;
                Task::none()
            }
            Message::SearchChanged(val) => {
                self.search_input = val;
                Task::none()
            }
            Message::SetFilterAll => {
                self.filter = PrFilter::All;
                self.selected_filter_label = "All PRs".to_string();
                self.do_refresh()
            }
            Message::SetFilterMine => {
                self.filter = PrFilter::Mine;
                self.selected_filter_label = "My PRs".to_string();
                self.do_refresh()
            }
            Message::SetFilterReview => {
                self.filter = PrFilter::ReviewRequested;
                self.selected_filter_label = "Review Requested".to_string();
                self.do_refresh()
            }
            Message::SetFilterByUser => {
                let user = self.user_filter_input.trim().to_string();
                if !user.is_empty() {
                    self.filter = PrFilter::ByUser(user);
                    self.selected_filter_label = "By User".to_string();
                    return self.do_refresh();
                }
                Task::none()
            }
            Message::SortBy(field) => {
                if self.sort_by == field {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_by = field;
                    self.sort_ascending = true;
                }
                self.sort_prs();
                Task::none()
            }
            // --- Actions ---
            Message::Refresh => self.do_refresh(),
            Message::ToggleAutoRefresh(val) => {
                self.auto_refresh = val;
                Task::none()
            }
            Message::ClickConnectionStatus => {
                let now = Instant::now();
                if let Some(last_time) = self.last_conn_click {
                    if now.duration_since(last_time).as_millis() < 500 {
                        self.last_conn_click = None;
                        self.active_tab = Tab::Settings;
                        return Task::none();
                    }
                }
                self.last_conn_click = Some(now);
                Task::none()
            }
            Message::ClickPr(index) => {
                let now = Instant::now();
                if let (Some(last_idx), Some(last_time)) =
                    (self.last_clicked_pr, self.last_click_time)
                {
                    if last_idx == index && now.duration_since(last_time).as_millis() < 500 {
                        if let Some(pr) = self.filtered_prs().get(index) {
                            let url = pr.html_url.clone();
                            self.last_clicked_pr = None;
                            self.last_click_time = None;
                            return Task::perform(
                                async move {
                                    let _ = open::that(&url);
                                    url
                                },
                                Message::OpenUrl,
                            );
                        }
                    }
                }
                self.last_clicked_pr = Some(index);
                self.last_click_time = Some(now);
                Task::none()
            }
            Message::OpenUrl(url) => {
                let _ = open::that(&url);
                Task::none()
            }
            // --- Tabs ---
            Message::SetTab(tab) => {
                self.active_tab = tab;
                if tab == Tab::RepoHealth && self.repo_health.is_empty() {
                    return self.do_refresh_health();
                }
                Task::none()
            }
            // --- Settings inputs ---
            Message::TokenChanged(val) => {
                self.token_input = val;
                Task::none()
            }
            Message::ToggleShowToken => {
                self.show_token = !self.show_token;
                Task::none()
            }
            Message::OAuthClientIdChanged(val) => {
                self.oauth_client_id_input = val;
                Task::none()
            }
            Message::SaveSettings => {
                self.config.token = self.token_input.clone();
                self.config.oauth_client_id = if self.oauth_client_id_input.trim().is_empty() {
                    None
                } else {
                    Some(self.oauth_client_id_input.trim().to_string())
                };
                let save_task = self.save_config();

                if !self.config.token.is_empty() && self.config.username.is_none() {
                    let token = self.config.token.clone();
                    let user_task =
                        Task::perform(client::fetch_username(token), Message::UsernameLoaded);
                    Task::batch([save_task, user_task])
                } else {
                    save_task
                }
            }
            // --- OAuth Device Flow ---
            Message::StartOAuth => {
                let client_id = if self.oauth_client_id_input.trim().is_empty() {
                    oauth::GITHUB_OAUTH_CLIENT_ID.to_string()
                } else {
                    self.oauth_client_id_input.trim().to_string()
                };

                if client_id == "REPLACE_WITH_YOUR_CLIENT_ID" || client_id.is_empty() {
                    self.oauth_status =
                        OAuthStatus::Error("Set your OAuth Client ID first".to_string());
                    return Task::none();
                }

                self.oauth_status = OAuthStatus::WaitingForUser;
                self.error = None;
                Task::perform(
                    async move { oauth::start_device_flow(&client_id).await },
                    Message::DeviceCodeReceived,
                )
            }
            Message::DeviceCodeReceived(result) => match result {
                Ok(resp) => {
                    self.oauth_user_code = Some(resp.user_code.clone());
                    self.oauth_verification_uri = Some(resp.verification_uri.clone());
                    self.oauth_status = OAuthStatus::Polling;
                    let _ = open::that(&resp.verification_uri);

                    let client_id = if self.oauth_client_id_input.trim().is_empty() {
                        oauth::GITHUB_OAUTH_CLIENT_ID.to_string()
                    } else {
                        self.oauth_client_id_input.trim().to_string()
                    };

                    Task::perform(
                        oauth::poll_for_token(client_id, resp.device_code, resp.interval),
                        Message::OAuthTokenReceived,
                    )
                }
                Err(e) => {
                    self.oauth_status = OAuthStatus::Error(e);
                    Task::none()
                }
            },
            Message::OAuthTokenReceived(result) => match result {
                Ok(token) => {
                    self.config.token = token.clone();
                    self.token_input = token.clone();
                    self.oauth_status = OAuthStatus::Idle;
                    self.oauth_user_code = None;
                    self.oauth_verification_uri = None;
                    self.status_message = Some("OAuth login successful!".to_string());

                    let save_task = self.save_config();
                    let user_task =
                        Task::perform(client::fetch_username(token), Message::UsernameLoaded);
                    Task::batch([save_task, user_task])
                }
                Err(e) => {
                    self.oauth_status = OAuthStatus::Error(e);
                    self.oauth_user_code = None;
                    self.oauth_verification_uri = None;
                    Task::none()
                }
            },
            Message::CancelOAuth => {
                self.oauth_status = OAuthStatus::Idle;
                self.oauth_user_code = None;
                self.oauth_verification_uri = None;
                Task::none()
            }
            // --- Async results (incremental) ---
            Message::ReposResolved(result) => match result {
                Ok(repos) => {
                    self.loading_repos_total = repos.len();
                    self.loading_repos_done = 0;
                    self.status_message =
                        Some(format!("Loading PRs from {} repo(s)...", repos.len()));

                    if repos.is_empty() {
                        self.loading = false;
                        self.status_message = Some("No repos to load".to_string());
                        return Task::none();
                    }

                    let token = self.config.token.clone();
                    let filter = self.filter.clone();
                    let username = self.config.username.clone();

                    let tasks: Vec<_> = repos
                        .into_iter()
                        .map(|repo| {
                            let t = token.clone();
                            let f = filter.clone();
                            let u = username.clone();
                            Task::perform(
                                client::fetch_repo_prs(t, repo, f, u),
                                |(repo, result)| Message::RepoPrsLoaded(repo, result),
                            )
                        })
                        .collect();

                    Task::batch(tasks)
                }
                Err(e) => {
                    self.loading = false;
                    self.error = Some(format!("Failed to resolve repos: {e}"));
                    Task::none()
                }
            },
            Message::RepoPrsLoaded(repo, result) => {
                self.loading_repos_done += 1;
                let done = self.loading_repos_done;
                let total = self.loading_repos_total;

                match result {
                    Ok(prs) => {
                        self.pull_requests.extend(prs);
                        self.sort_prs();
                        self.error = None;
                        let pr_count = self.pull_requests.len();
                        if done >= total {
                            self.loading = false;
                            self.status_message =
                                Some(format!("Loaded {pr_count} PRs from {total} repo(s)"));
                        } else {
                            self.status_message =
                                Some(format!("{pr_count} PRs ({done}/{total} repos)..."));
                        }
                    }
                    Err(e) => {
                        if done >= total {
                            self.loading = false;
                        }
                        self.error = Some(format!("{repo}: {e}"));
                    }
                }
                Task::none()
            }
            Message::RepoHealthLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(health) => {
                        self.repo_health = health;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                Task::none()
            }
            Message::UsernameLoaded(result) => {
                match result {
                    Ok(username) => {
                        self.config.username = Some(username);
                        self.status_message = Some("Authenticated".to_string());
                        return self.do_refresh();
                    }
                    Err(e) => {
                        self.loading = false;
                        self.error = Some(format!("Auth failed: {e}"));
                    }
                }
                Task::none()
            }
            Message::SavedConfig(_) => Task::none(),
            Message::Tick(_) => self.do_refresh(),
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn has_sources(&self) -> bool {
        !self.config.repos.is_empty() || !self.config.orgs.is_empty()
    }

    fn do_refresh(&mut self) -> Task<Message> {
        if self.config.token.is_empty() || !self.has_sources() {
            return Task::none();
        }
        self.loading = true;
        self.error = None;
        self.pull_requests.clear();
        self.loading_repos_total = 0;
        self.loading_repos_done = 0;
        self.status_message = Some("Resolving repositories...".to_string());

        let token = self.config.token.clone();
        let repos = self.config.repos.clone();
        let orgs = self.config.orgs.clone();

        let resolve_task = Task::perform(
            client::resolve_repos(token.clone(), repos.clone(), orgs.clone()),
            Message::ReposResolved,
        );

        if self.active_tab == Tab::RepoHealth {
            let health_task = Task::perform(
                client::fetch_repo_health(token, repos, orgs),
                Message::RepoHealthLoaded,
            );
            Task::batch([resolve_task, health_task])
        } else {
            resolve_task
        }
    }

    fn do_refresh_health(&mut self) -> Task<Message> {
        if self.config.token.is_empty() || !self.has_sources() {
            return Task::none();
        }
        self.loading = true;
        let token = self.config.token.clone();
        let repos = self.config.repos.clone();
        let orgs = self.config.orgs.clone();
        Task::perform(
            client::fetch_repo_health(token, repos, orgs),
            Message::RepoHealthLoaded,
        )
    }

    fn save_config(&self) -> Task<Message> {
        let config = self.config.clone();
        Task::perform(async move { config.save() }, Message::SavedConfig)
    }

    fn sort_prs(&mut self) {
        let asc = self.sort_ascending;
        self.pull_requests.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortField::Repo => a.repo_full_name.cmp(&b.repo_full_name),
                SortField::Number => a.number.cmp(&b.number),
                SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortField::Author => a.author.to_lowercase().cmp(&b.author.to_lowercase()),
                SortField::CiStatus => a.ci_status.sort_order().cmp(&b.ci_status.sort_order()),
                SortField::Age => a.created_at.cmp(&b.created_at),
                SortField::Size => (a.additions + a.deletions).cmp(&(b.additions + b.deletions)),
            };
            if asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    fn filtered_prs(&self) -> Vec<&PullRequest> {
        let search = self.search_input.to_lowercase();
        self.pull_requests
            .iter()
            .filter(|pr| {
                if search.is_empty() {
                    return true;
                }
                pr.title.to_lowercase().contains(&search)
                    || pr.author.to_lowercase().contains(&search)
                    || pr.repo_full_name.to_lowercase().contains(&search)
                    || pr.number.to_string().contains(&search)
                    || pr
                        .labels
                        .iter()
                        .any(|(name, _)| name.to_lowercase().contains(&search))
            })
            .collect()
    }

    fn is_connected(&self) -> bool {
        self.config.username.is_some() && !self.config.token.is_empty()
    }

    // -----------------------------------------------------------------------
    // View - top level
    // -----------------------------------------------------------------------

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let content = match self.active_tab {
            Tab::PullRequests => self.view_pr_tab(),
            Tab::RepoHealth => self.view_health_tab(),
            Tab::Settings => self.view_settings_tab(),
        };

        let main_layout = row![
            container(sidebar)
                .width(260)
                .height(Length::Fill)
                .padding(10)
                .style(container::bordered_box),
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10),
        ]
        .spacing(0);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    // -----------------------------------------------------------------------
    // Sidebar (filters, actions) + connection status at bottom
    // -----------------------------------------------------------------------

    fn view_sidebar(&self) -> Element<'_, Message> {
        let title = text("Gituqueiro").size(22);

        // Sources summary
        let repo_count = self.config.repos.len();
        let org_count = self.config.orgs.len();
        let repos_label: Element<'_, Message> = if repo_count == 0 && org_count == 0 {
            button(
                text("No sources configured - go to Settings")
                    .size(11)
                    .color(color!(0xFF8844)),
            )
            .on_press(Message::SetTab(Tab::Settings))
            .style(button::text)
            .padding(0)
            .into()
        } else {
            let mut parts = Vec::new();
            if repo_count > 0 {
                parts.push(format!("{repo_count} repo(s)"));
            }
            if org_count > 0 {
                parts.push(format!("{org_count} org(s)"));
            }
            text(format!("Monitoring {}", parts.join(", ")))
                .size(11)
                .color(color!(0x888888))
                .into()
        };

        // Filters
        let filter_label = text("Filter:").size(13);

        let filter_all = button(text("All PRs").size(12))
            .on_press(Message::SetFilterAll)
            .width(Length::Fill)
            .style(if self.selected_filter_label == "All PRs" {
                button::primary
            } else {
                button::secondary
            });

        let filter_mine = button(text("My PRs").size(12))
            .on_press(Message::SetFilterMine)
            .width(Length::Fill)
            .style(if self.selected_filter_label == "My PRs" {
                button::primary
            } else {
                button::secondary
            });

        let filter_review = button(text("Review Requested").size(12))
            .on_press(Message::SetFilterReview)
            .width(Length::Fill)
            .style(if self.selected_filter_label == "Review Requested" {
                button::primary
            } else {
                button::secondary
            });

        let user_input = text_input("username", &self.user_filter_input)
            .on_input(Message::UserFilterChanged)
            .on_submit(Message::SetFilterByUser)
            .size(12);
        let user_btn = button(text("Filter").size(12))
            .on_press(Message::SetFilterByUser)
            .style(if self.selected_filter_label == "By User" {
                button::primary
            } else {
                button::secondary
            });
        let user_row = row![user_input, user_btn].spacing(4);

        // Actions
        let auto_check = checkbox("Auto-refresh", self.auto_refresh)
            .on_toggle(Message::ToggleAutoRefresh)
            .size(16);

        let refresh_btn = button(
            text(if self.loading {
                "Loading..."
            } else {
                "Refresh"
            })
            .size(14),
        )
        .on_press(Message::Refresh)
        .width(Length::Fill)
        .style(button::primary);

        // Status message
        let status_msg: Element<'_, Message> = if let Some(ref err) = self.error {
            text(err).size(10).color(color!(0xFF6666)).into()
        } else if let Some(ref msg) = self.status_message {
            text(msg).size(10).color(color!(0x88CC88)).into()
        } else {
            Space::with_height(0).into()
        };

        // Connection status (bottom)
        let (dot_color, status_text) = if self.is_connected() {
            (
                color!(0x44CC44),
                self.config.username.as_deref().unwrap_or("").to_string(),
            )
        } else if self.oauth_status == OAuthStatus::Polling
            || self.oauth_status == OAuthStatus::WaitingForUser
        {
            (color!(0xFFCC44), "Authenticating...".to_string())
        } else {
            (color!(0xFF4444), "Not connected".to_string())
        };

        let connection_status = button(
            row![
                text("●").size(12).color(dot_color),
                text(status_text).size(11).color(color!(0xAABBCC)),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ClickConnectionStatus)
        .style(button::text)
        .padding([4, 0])
        .width(Length::Fill);

        let sidebar_content = column![
            title,
            repos_label,
            Space::with_height(8),
            horizontal_rule(1),
            Space::with_height(4),
            filter_label,
            filter_all,
            filter_mine,
            filter_review,
            text("By user:").size(12),
            user_row,
            Space::with_height(8),
            horizontal_rule(1),
            Space::with_height(4),
            auto_check,
            Space::with_height(4),
            refresh_btn,
            status_msg,
        ]
        .spacing(4)
        .width(Length::Fill);

        column![
            scrollable(sidebar_content).height(Length::Fill),
            horizontal_rule(1),
            Space::with_height(4),
            connection_status,
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    // -----------------------------------------------------------------------
    // Tab bar (shared across content views)
    // -----------------------------------------------------------------------

    fn view_tab_bar(&self) -> Element<'_, Message> {
        let tab_style = |tab: Tab| {
            if self.active_tab == tab {
                button::primary
            } else {
                button::secondary
            }
        };

        row![
            button(text("Pull Requests").size(14))
                .on_press(Message::SetTab(Tab::PullRequests))
                .style(tab_style(Tab::PullRequests))
                .padding([4, 12]),
            button(text("Security & Quality").size(14))
                .on_press(Message::SetTab(Tab::RepoHealth))
                .style(tab_style(Tab::RepoHealth))
                .padding([4, 12]),
            Space::with_width(Length::Fill),
            button(text("Settings").size(14))
                .on_press(Message::SetTab(Tab::Settings))
                .style(tab_style(Tab::Settings))
                .padding([4, 12]),
        ]
        .spacing(4)
        .into()
    }

    // -----------------------------------------------------------------------
    // Settings tab
    // -----------------------------------------------------------------------

    fn view_settings_tab(&self) -> Element<'_, Message> {
        let tab_bar = self.view_tab_bar();

        // === Connection status ===
        let conn_section = if self.is_connected() {
            let username = self.config.username.as_deref().unwrap_or("unknown");
            row![
                text("●").size(14).color(color!(0x44CC44)),
                text(format!("Connected as {username}"))
                    .size(14)
                    .color(color!(0x88CC88)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        } else {
            row![
                text("●").size(14).color(color!(0xFF4444)),
                text("Not connected").size(14).color(color!(0xFF6666)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        };

        // === Authentication ===
        let auth_title = text("Authentication").size(18);

        let token_label = text("GitHub Token (personal access token):").size(13);
        let token_input = if self.show_token {
            text_input("ghp_...", &self.token_input)
                .on_input(Message::TokenChanged)
                .size(13)
                .width(Length::Fill)
        } else {
            text_input("ghp_...", &self.token_input)
                .on_input(Message::TokenChanged)
                .secure(true)
                .size(13)
                .width(Length::Fill)
        };
        let eye_btn = button(text(if self.show_token { "Hide" } else { "Show" }).size(12))
            .on_press(Message::ToggleShowToken)
            .padding([2, 6]);
        let token_row = row![token_input, eye_btn].spacing(4);

        let save_btn = button(text("Save & Connect").size(13))
            .on_press(Message::SaveSettings)
            .padding([4, 16])
            .style(button::primary);

        let or_divider = container(
            text("-- or login via OAuth --")
                .size(12)
                .color(color!(0x666666)),
        )
        .width(Length::Fill)
        .center_x(Length::Fill);

        // OAuth
        let oauth_title = text("OAuth Device Flow").size(16);
        let client_id_label = text("OAuth App Client ID:").size(13);
        let client_id_input = text_input("Iv1.xxxxxxxxx", &self.oauth_client_id_input)
            .on_input(Message::OAuthClientIdChanged)
            .size(13)
            .width(Length::Fill);
        let client_id_hint = text(
            "Create a GitHub OAuth App (Settings > Developer settings > OAuth Apps), enable Device Flow",
        )
        .size(10)
        .color(color!(0x666666));

        let mut oauth_section = column![
            oauth_title,
            Space::with_height(4),
            client_id_label,
            client_id_input,
            client_id_hint,
            Space::with_height(8),
        ]
        .spacing(4);

        match &self.oauth_status {
            OAuthStatus::Idle => {
                oauth_section = oauth_section.push(
                    button(text("Login with GitHub").size(13))
                        .on_press(Message::StartOAuth)
                        .padding([4, 16])
                        .style(button::primary),
                );
            }
            OAuthStatus::WaitingForUser => {
                oauth_section = oauth_section.push(
                    text("Starting OAuth flow...")
                        .size(12)
                        .color(color!(0xFFCC44)),
                );
            }
            OAuthStatus::Polling => {
                if let Some(ref code) = self.oauth_user_code {
                    let code_box = container(
                        column![
                            text("Enter this code in your browser:").size(13),
                            Space::with_height(8),
                            text(code).size(28).color(color!(0xFFCC44)),
                        ]
                        .spacing(0)
                        .align_x(iced::Alignment::Center),
                    )
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .padding(16)
                    .style(container::bordered_box);

                    oauth_section = oauth_section.push(code_box);

                    if let Some(ref uri) = self.oauth_verification_uri {
                        oauth_section = oauth_section.push(
                            text(format!("Browser opened to {uri}"))
                                .size(11)
                                .color(color!(0x888888)),
                        );
                    }
                    oauth_section = oauth_section.push(
                        text("Waiting for you to authorize...")
                            .size(12)
                            .color(color!(0xFFCC44)),
                    );
                }
                oauth_section = oauth_section.push(Space::with_height(4));
                oauth_section = oauth_section.push(
                    button(text("Cancel").size(12))
                        .on_press(Message::CancelOAuth)
                        .padding([4, 16])
                        .style(button::danger),
                );
            }
            OAuthStatus::Error(err) => {
                oauth_section = oauth_section.push(text(err).size(12).color(color!(0xFF6666)));
                oauth_section = oauth_section.push(Space::with_height(4));
                oauth_section = oauth_section.push(
                    button(text("Try Again").size(13))
                        .on_press(Message::StartOAuth)
                        .padding([4, 16])
                        .style(button::primary),
                );
            }
        }

        // === Monitored Sources ===
        let sources_title = text("Monitored Sources").size(18);

        // -- Individual Repos --
        let repos_subtitle = text("Repositories").size(15);
        let repos_hint = text("Add individual repositories (e.g. owner/repo)")
            .size(10)
            .color(color!(0x666666));
        let repo_input = text_input("owner/repo", &self.repo_input)
            .on_input(Message::RepoInputChanged)
            .on_submit(Message::AddRepo)
            .size(13)
            .width(Length::Fill);
        let add_repo_btn = button(text("+ Add").size(13))
            .on_press(Message::AddRepo)
            .padding([4, 12])
            .style(button::primary);
        let repo_add_row = row![repo_input, add_repo_btn].spacing(8);

        let mut repo_list = column![].spacing(4);
        if self.config.repos.is_empty() {
            repo_list = repo_list.push(
                text("No repositories added")
                    .size(12)
                    .color(color!(0x888888)),
            );
        }
        for (i, repo) in self.config.repos.iter().enumerate() {
            let remove_btn = button(text("Remove").size(11))
                .on_press(Message::RemoveRepo(i))
                .padding([2, 8])
                .style(button::danger);
            let repo_entry = container(
                row![text(repo).size(13).width(Length::Fill), remove_btn,]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            )
            .padding([8, 12])
            .width(Length::Fill)
            .style(container::bordered_box);
            repo_list = repo_list.push(repo_entry);
        }

        // -- Organizations --
        let orgs_subtitle = text("Organizations").size(15);
        let orgs_hint = text("Monitor all repos in an organization (e.g. my-org)")
            .size(10)
            .color(color!(0x666666));
        let org_input = text_input("organization", &self.org_input)
            .on_input(Message::OrgInputChanged)
            .on_submit(Message::AddOrg)
            .size(13)
            .width(Length::Fill);
        let add_org_btn = button(text("+ Add").size(13))
            .on_press(Message::AddOrg)
            .padding([4, 12])
            .style(button::primary);
        let org_add_row = row![org_input, add_org_btn].spacing(8);

        let mut org_list = column![].spacing(4);
        if self.config.orgs.is_empty() {
            org_list = org_list.push(
                text("No organizations added")
                    .size(12)
                    .color(color!(0x888888)),
            );
        }
        for (i, org) in self.config.orgs.iter().enumerate() {
            let remove_btn = button(text("Remove").size(11))
                .on_press(Message::RemoveOrg(i))
                .padding([2, 8])
                .style(button::danger);
            let org_entry = container(
                row![
                    text(format!("{org} (all repos)"))
                        .size(13)
                        .width(Length::Fill),
                    remove_btn,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding([8, 12])
            .width(Length::Fill)
            .style(container::bordered_box);
            org_list = org_list.push(org_entry);
        }

        // === Preferences ===
        let prefs_title = text("Preferences").size(18);
        let auto_check = checkbox("Auto-refresh", self.auto_refresh)
            .on_toggle(Message::ToggleAutoRefresh)
            .size(16);

        // === Assemble ===
        let settings_content = column![
            conn_section,
            Space::with_height(20),
            auth_title,
            Space::with_height(8),
            token_label,
            token_row,
            Space::with_height(8),
            save_btn,
            Space::with_height(16),
            or_divider,
            Space::with_height(16),
            oauth_section,
            Space::with_height(24),
            horizontal_rule(1),
            Space::with_height(16),
            sources_title,
            Space::with_height(12),
            repos_subtitle,
            repos_hint,
            Space::with_height(4),
            repo_add_row,
            Space::with_height(4),
            repo_list,
            Space::with_height(16),
            orgs_subtitle,
            orgs_hint,
            Space::with_height(4),
            org_add_row,
            Space::with_height(4),
            org_list,
            Space::with_height(24),
            horizontal_rule(1),
            Space::with_height(16),
            prefs_title,
            Space::with_height(8),
            auto_check,
        ]
        .spacing(4)
        .width(Length::Fill)
        .max_width(600);

        column![
            tab_bar,
            Space::with_height(8),
            scrollable(
                container(settings_content)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .padding(20)
            )
            .height(Length::Fill),
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    // -----------------------------------------------------------------------
    // Pull Requests tab
    // -----------------------------------------------------------------------

    fn view_pr_tab(&self) -> Element<'_, Message> {
        let tab_bar = self.view_tab_bar();

        let search = text_input("Search PRs...", &self.search_input)
            .on_input(Message::SearchChanged)
            .size(13)
            .width(Length::Fill);

        let header = self.view_pr_header();

        let prs = self.filtered_prs();
        let mut pr_list = column![].spacing(1);

        if prs.is_empty() && !self.loading {
            pr_list = pr_list.push(
                container(
                    text(if self.config.repos.is_empty() {
                        "Add repositories in the sidebar to get started"
                    } else if self.config.token.is_empty() {
                        "Go to Settings to configure authentication"
                    } else {
                        "No pull requests found"
                    })
                    .size(14)
                    .color(color!(0x888888)),
                )
                .padding(20)
                .width(Length::Fill)
                .center_x(Length::Fill),
            );
        }

        for (i, pr) in prs.iter().enumerate() {
            pr_list = pr_list.push(self.view_pr_row(i, pr));
        }

        let total = self.pull_requests.len();
        let failing = self
            .pull_requests
            .iter()
            .filter(|p| p.ci_status == CiStatus::Failure)
            .count();
        let stale = self.pull_requests.iter().filter(|p| p.is_stale()).count();
        let drafts = self.pull_requests.iter().filter(|p| p.draft).count();

        let stats = row![
            text(format!("Total: {total}"))
                .size(11)
                .color(color!(0xAABBCC)),
            text("|").size(11).color(color!(0x555555)),
            text(format!("Failing: {failing}"))
                .size(11)
                .color(if failing > 0 {
                    color!(0xFF6666)
                } else {
                    color!(0x88CC88)
                }),
            text("|").size(11).color(color!(0x555555)),
            text(format!("Stale: {stale}"))
                .size(11)
                .color(if stale > 0 {
                    color!(0xFFAA44)
                } else {
                    color!(0xAABBCC)
                }),
            text("|").size(11).color(color!(0x555555)),
            text(format!("Drafts: {drafts}"))
                .size(11)
                .color(color!(0xAABBCC)),
        ]
        .spacing(6);

        column![
            tab_bar,
            search,
            Space::with_height(4),
            header,
            scrollable(pr_list).height(Length::Fill),
            Space::with_height(4),
            horizontal_rule(1),
            stats,
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_pr_header(&self) -> Element<'_, Message> {
        let make_header = |label: &str, field: SortField, width: u16| -> Element<Message> {
            let arrow = if self.sort_by == field {
                if self.sort_ascending {
                    " ^"
                } else {
                    " v"
                }
            } else {
                ""
            };
            button(text(format!("{label}{arrow}")).size(11))
                .on_press(Message::SortBy(field))
                .padding([2, 4])
                .width(width)
                .style(button::secondary)
                .into()
        };

        row![
            make_header("Repo", SortField::Repo, 140),
            make_header("#", SortField::Number, 50),
            make_header("Title", SortField::Title, 300),
            make_header("Author", SortField::Author, 110),
            make_header("CI", SortField::CiStatus, 50),
            make_header("Age", SortField::Age, 50),
            make_header("Size", SortField::Size, 50),
        ]
        .spacing(2)
        .into()
    }

    fn view_pr_row<'a>(&self, index: usize, pr: &'a PullRequest) -> Element<'a, Message> {
        let age_color = if pr.age_days() > 30 {
            color!(0xFF6666)
        } else if pr.age_days() > 7 {
            color!(0xFFAA44)
        } else {
            color!(0x88CC88)
        };

        let ci_color = match pr.ci_status {
            CiStatus::Success => color!(0x88CC88),
            CiStatus::Failure => color!(0xFF6666),
            CiStatus::Running | CiStatus::Pending => color!(0xFFCC44),
            CiStatus::Unknown => color!(0x888888),
        };

        let draft_indicator = if pr.draft { "[DRAFT] " } else { "" };

        let title_text = text(format!("{draft_indicator}{}", pr.title))
            .size(12)
            .width(300);
        let title_with_labels: Element<Message> = if pr.labels.is_empty() {
            title_text.into()
        } else {
            let label_str: String = pr
                .labels
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tooltip(
                title_text,
                text(format!("Labels: {label_str}")).size(11),
                tooltip::Position::Bottom,
            )
            .into()
        };

        let ci_element: Element<Message> = if let Some(ref url) = pr.ci_url {
            let url = url.clone();
            let ci_btn = button(text(pr.ci_status.symbol()).size(12).color(ci_color))
                .on_press(Message::OpenUrl(url))
                .padding([0, 4])
                .style(button::text);

            let checks_info: String = pr
                .checks
                .iter()
                .map(|c| format!("{}: {}", c.name, c.status.symbol()))
                .collect::<Vec<_>>()
                .join("\n");

            if checks_info.is_empty() {
                ci_btn.width(50).into()
            } else {
                tooltip(
                    ci_btn.width(50),
                    text(checks_info).size(10),
                    tooltip::Position::Left,
                )
                .into()
            }
        } else {
            text(pr.ci_status.symbol())
                .size(12)
                .color(ci_color)
                .width(50)
                .into()
        };

        let size_color = match pr.size_label() {
            "XS" | "S" => color!(0x88CC88),
            "M" => color!(0xFFCC44),
            _ => color!(0xFF6666),
        };

        let pr_row = button(
            row![
                text(&pr.repo_full_name).size(12).width(140),
                text(pr.number.to_string()).size(12).width(50),
                title_with_labels,
                text(&pr.author).size(12).width(110),
                ci_element,
                text(pr.age_display()).size(12).color(age_color).width(50),
                text(pr.size_label()).size(12).color(size_color).width(50),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ClickPr(index))
        .padding([4, 4])
        .width(Length::Fill)
        .style(button::text);

        pr_row.into()
    }

    // -----------------------------------------------------------------------
    // Security & Quality tab
    // -----------------------------------------------------------------------

    fn view_health_tab(&self) -> Element<'_, Message> {
        let tab_bar = self.view_tab_bar();

        let mut content = column![].spacing(12);

        if self.repo_health.is_empty() && !self.loading {
            content = content.push(
                container(
                    text("No health data loaded. Click Refresh.")
                        .size(14)
                        .color(color!(0x888888)),
                )
                .padding(20)
                .width(Length::Fill)
                .center_x(Length::Fill),
            );
        }

        for health in &self.repo_health {
            content = content.push(self.view_repo_health_card(health));
        }

        column![tab_bar, scrollable(content).height(Length::Fill),]
            .spacing(4)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_repo_health_card<'a>(&self, health: &'a RepoHealth) -> Element<'a, Message> {
        let critical = health.critical_count();
        let title_color = if critical > 0 {
            color!(0xFF6666)
        } else {
            color!(0x88CC88)
        };

        let repo_url = format!("https://github.com/{}", health.repo_full_name);

        let header = row![
            button(text(&health.repo_full_name).size(16).color(title_color),)
                .on_press(Message::OpenUrl(repo_url))
                .style(button::text)
                .padding(0),
            Space::with_width(Length::Fill),
            text(format!("Open PRs: {}", health.open_prs_count))
                .size(12)
                .color(color!(0xAABBCC)),
            text(format!("Stale PRs: {}", health.stale_prs_count))
                .size(12)
                .color(if health.stale_prs_count > 0 {
                    color!(0xFFAA44)
                } else {
                    color!(0xAABBCC)
                }),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let code_scan_critical = health
            .code_scanning_alerts
            .iter()
            .filter(|a| a.severity == "critical" || a.severity == "high")
            .count();
        let code_scan_total = health.code_scanning_alerts.len();

        let dep_critical = health
            .dependabot_alerts
            .iter()
            .filter(|a| a.severity == "critical" || a.severity == "high")
            .count();
        let dep_total = health.dependabot_alerts.len();

        let security_row = row![
            text(format!(
                "Code Scanning: {code_scan_total} ({code_scan_critical} critical/high)"
            ))
            .size(12)
            .color(if code_scan_critical > 0 {
                color!(0xFF6666)
            } else {
                color!(0xAABBCC)
            }),
            text("|").size(12).color(color!(0x555555)),
            text(format!(
                "Dependabot: {dep_total} ({dep_critical} critical/high)"
            ))
            .size(12)
            .color(if dep_critical > 0 {
                color!(0xFF6666)
            } else {
                color!(0xAABBCC)
            }),
        ]
        .spacing(8);

        let mut alert_list = column![].spacing(2);
        let mut all_alerts: Vec<(&str, &str, &str, &str)> = Vec::new();

        for a in &health.code_scanning_alerts {
            all_alerts.push((&a.severity, &a.description, &a.url, "Code Scan"));
        }
        for a in &health.dependabot_alerts {
            all_alerts.push((&a.severity, &a.summary, &a.url, "Dependabot"));
        }

        all_alerts.sort_by(|a, b| severity_order(a.0).cmp(&severity_order(b.0)));

        for (severity, desc, url, source) in all_alerts.iter().take(5) {
            let sev_color = match *severity {
                "critical" => color!(0xFF4444),
                "high" => color!(0xFF8844),
                "medium" => color!(0xFFCC44),
                _ => color!(0xAABBCC),
            };
            let url_owned = url.to_string();
            let alert_row = row![
                text(format!("[{severity}]"))
                    .size(11)
                    .color(sev_color)
                    .width(70),
                text(format!("{source}: {desc}"))
                    .size(11)
                    .width(Length::Fill),
                button(text("View").size(10))
                    .on_press(Message::OpenUrl(url_owned))
                    .padding([1, 4])
                    .style(button::secondary),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center);
            alert_list = alert_list.push(alert_row);
        }

        if all_alerts.len() > 5 {
            alert_list = alert_list.push(
                text(format!("... and {} more alerts", all_alerts.len() - 5))
                    .size(11)
                    .color(color!(0x888888)),
            );
        }

        let card = column![header, security_row, alert_list, horizontal_rule(1),]
            .spacing(4)
            .padding(8);

        container(card)
            .width(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}

fn severity_order(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}
