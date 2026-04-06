use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub token: String,
    pub repos: Vec<String>,
    #[serde(default)]
    pub orgs: Vec<String>,
    pub auto_refresh_seconds: u64,
    pub username: Option<String>,
    #[serde(default)]
    pub oauth_client_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").unwrap_or_default(),
            repos: vec![],
            orgs: vec![],
            auto_refresh_seconds: 300,
            username: None,
            oauth_client_id: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("gituqueiro");
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let data = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, data).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.repos.is_empty());
        assert_eq!(config.auto_refresh_seconds, 300);
        assert!(config.username.is_none());
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config {
            token: "ghp_test123".to_string(),
            repos: vec!["owner/repo1".to_string(), "owner/repo2".to_string()],
            auto_refresh_seconds: 120,
            username: Some("testuser".to_string()),
            orgs: vec![],
            oauth_client_id: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.token, "ghp_test123");
        assert_eq!(deserialized.repos.len(), 2);
        assert_eq!(deserialized.auto_refresh_seconds, 120);
        assert_eq!(deserialized.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("gituqueiro_test");
        let path = dir.join("config.json");

        let config = Config {
            token: "test_token".to_string(),
            repos: vec!["test/repo".to_string()],
            orgs: vec![],
            auto_refresh_seconds: 60,
            username: Some("testuser".to_string()),
            oauth_client_id: None,
        };

        fs::create_dir_all(&dir).unwrap();
        let data = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&path, &data).unwrap();

        let loaded: Config = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.token, "test_token");
        assert_eq!(loaded.repos, vec!["test/repo".to_string()]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_config_path_not_empty() {
        let path = Config::config_path();
        assert!(path.to_str().unwrap().contains("gituqueiro"));
    }
}
