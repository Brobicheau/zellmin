use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub worktree_dir_name: String,
    pub session_prefix: Option<String>,
    pub base_branch: Option<String>,
    pub remote: String,
    pub auto_fetch: bool,
    pub worktree_naming_pattern: WorktreeNamingPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeNamingPattern {
    Branch,
    Hash,
    BranchHash,
}

#[derive(Debug, Default, Deserialize)]
struct RepoConfig {
    worktree_dir_name: Option<String>,
    session_prefix: Option<String>,
    base_branch: Option<String>,
    remote: Option<String>,
    auto_fetch: Option<bool>,
    worktree_naming_pattern: Option<WorktreeNamingPattern>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worktree_dir_name: ".worktrees".to_string(),
            session_prefix: None,
            base_branch: None,
            remote: "origin".to_string(),
            auto_fetch: false,
            worktree_naming_pattern: WorktreeNamingPattern::Branch,
        }
    }
}

impl Config {
    pub fn from_kdl(configuration: BTreeMap<String, String>) -> Self {
        let mut config = Self::default();

        if let Some(worktree_dir_name) = configuration.get("worktree_dir_name") {
            config.worktree_dir_name = trim_to_option(worktree_dir_name)
                .unwrap_or_else(|| config.worktree_dir_name.clone());
        }

        if let Some(session_prefix) = configuration.get("session_prefix") {
            config.session_prefix = trim_to_option(session_prefix);
        }

        if let Some(base_branch) = configuration.get("base_branch") {
            config.base_branch = trim_to_option(base_branch);
        }

        if let Some(remote) = configuration.get("remote") {
            config.remote = trim_to_option(remote).unwrap_or_else(|| config.remote.clone());
        }

        if let Some(auto_fetch) = configuration.get("auto_fetch") {
            config.auto_fetch = auto_fetch.trim().eq_ignore_ascii_case("true");
        }

        if let Some(pattern) = configuration.get("worktree_naming_pattern") {
            if let Some(pattern) = WorktreeNamingPattern::from_str(pattern) {
                config.worktree_naming_pattern = pattern;
            }
        }

        config
    }

    pub fn from_toml(toml_content: &str) -> Result<Self, String> {
        let repo_config: RepoConfig = toml::from_str(toml_content)
            .map_err(|error| format!("Failed to parse .treemin.toml: {error}"))?;

        let mut config = Self::default();
        config.apply_repo_config(repo_config);
        Ok(config)
    }

    pub fn merge(&mut self, repo_config: Config) {
        let default = Config::default();

        if repo_config.worktree_dir_name != default.worktree_dir_name {
            self.worktree_dir_name = repo_config.worktree_dir_name;
        }

        if repo_config.session_prefix != default.session_prefix {
            self.session_prefix = repo_config.session_prefix;
        }

        if repo_config.base_branch != default.base_branch {
            self.base_branch = repo_config.base_branch;
        }

        if repo_config.remote != default.remote {
            self.remote = repo_config.remote;
        }

        if repo_config.auto_fetch != default.auto_fetch {
            self.auto_fetch = repo_config.auto_fetch;
        }

        if repo_config.worktree_naming_pattern != default.worktree_naming_pattern {
            self.worktree_naming_pattern = repo_config.worktree_naming_pattern;
        }
    }

    fn apply_repo_config(&mut self, repo_config: RepoConfig) {
        if let Some(worktree_dir_name) = repo_config.worktree_dir_name.and_then(|value| trim_to_option(&value)) {
            self.worktree_dir_name = worktree_dir_name;
        }

        if let Some(session_prefix) = repo_config.session_prefix {
            self.session_prefix = trim_to_option(&session_prefix);
        }

        if let Some(base_branch) = repo_config.base_branch {
            self.base_branch = trim_to_option(&base_branch);
        }

        if let Some(remote) = repo_config.remote.and_then(|value| trim_to_option(&value)) {
            self.remote = remote;
        }

        if let Some(auto_fetch) = repo_config.auto_fetch {
            self.auto_fetch = auto_fetch;
        }

        if let Some(worktree_naming_pattern) = repo_config.worktree_naming_pattern {
            self.worktree_naming_pattern = worktree_naming_pattern;
        }
    }
}

impl WorktreeNamingPattern {
    fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "branch" => Some(Self::Branch),
            "hash" => Some(Self::Hash),
            "branch-hash" => Some(Self::BranchHash),
            _ => None,
        }
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Config, WorktreeNamingPattern};

    #[test]
    fn default_config_matches_existing_behavior() {
        let config = Config::default();

        assert_eq!(config.worktree_dir_name, ".worktrees");
        assert_eq!(config.session_prefix, None);
        assert_eq!(config.base_branch, None);
        assert_eq!(config.remote, "origin");
        assert!(!config.auto_fetch);
        assert_eq!(config.worktree_naming_pattern, WorktreeNamingPattern::Branch);
    }

    #[test]
    fn kdl_config_parses_known_fields() {
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), "trees".to_string());
        kdl.insert("session_prefix".to_string(), "wt".to_string());
        kdl.insert("base_branch".to_string(), "main".to_string());
        kdl.insert("remote".to_string(), "upstream".to_string());
        kdl.insert("auto_fetch".to_string(), "true".to_string());
        kdl.insert(
            "worktree_naming_pattern".to_string(),
            "branch-hash".to_string(),
        );

        let config = Config::from_kdl(kdl);

        assert_eq!(config.worktree_dir_name, "trees");
        assert_eq!(config.session_prefix, Some("wt".to_string()));
        assert_eq!(config.base_branch, Some("main".to_string()));
        assert_eq!(config.remote, "upstream");
        assert!(config.auto_fetch);
        assert_eq!(config.worktree_naming_pattern, WorktreeNamingPattern::BranchHash);
    }

    #[test]
    fn toml_config_can_be_partial() {
        let config = Config::from_toml(
            r#"
worktree_dir_name = "repo-trees"
base_branch = "main"
"#,
        )
        .unwrap();

        assert_eq!(config.worktree_dir_name, "repo-trees");
        assert_eq!(config.base_branch, Some("main".to_string()));
        assert_eq!(config.remote, "origin");
        assert!(!config.auto_fetch);
    }

    #[test]
    fn repo_config_overrides_kdl_config() {
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), "kdl-trees".to_string());
        kdl.insert("session_prefix".to_string(), "kdl".to_string());

        let mut config = Config::from_kdl(kdl);
        let repo_config = Config::from_toml(
            r#"
worktree_dir_name = "repo-trees"
base_branch = "main"
"#,
        )
        .unwrap();

        config.merge(repo_config);

        assert_eq!(config.worktree_dir_name, "repo-trees");
        assert_eq!(config.session_prefix, Some("kdl".to_string()));
        assert_eq!(config.base_branch, Some("main".to_string()));
    }
}
