use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Directory name for worktrees relative to repo root (e.g., ".worktrees")
    pub worktree_dir_name: String,
    
    /// Optional prefix for Zellij session names (e.g., "wt" -> "wt-repo-branch-hash")
    pub session_prefix: Option<String>,
    
    /// Base branch to track from when creating new branches (e.g., "main", "develop")
    pub base_branch: Option<String>,
    
    /// Git remote to use when checking out branches (default: "origin")
    pub remote: String,
    
    /// Whether to fetch from remote before creating worktree
    pub auto_fetch: bool,
    
    /// Pattern for worktree directory naming: "branch", "hash", or "branch-hash"
    pub worktree_naming_pattern: WorktreeNamingPattern,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeNamingPattern {
    /// Use sanitized branch name as-is
    Branch,
    /// Use only the hash of the branch name
    Hash,
    /// Use "branch-hash" format (default)
    BranchHash,
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
    /// Create config from Zellij plugin configuration (KDL)
    pub fn from_kdl(configuration: BTreeMap<String, String>) -> Self {
        let mut config = Self::default();
        
        if let Some(worktree_dir_name) = configuration.get("worktree_dir_name") {
            let trimmed = worktree_dir_name.trim();
            if !trimmed.is_empty() {
                config.worktree_dir_name = trimmed.to_string();
            }
        }
        
        if let Some(session_prefix) = configuration.get("session_prefix") {
            let trimmed = session_prefix.trim();
            if !trimmed.is_empty() {
                config.session_prefix = Some(trimmed.to_string());
            }
        }
        
        if let Some(base_branch) = configuration.get("base_branch") {
            let trimmed = base_branch.trim();
            if !trimmed.is_empty() {
                config.base_branch = Some(trimmed.to_string());
            }
        }
        
        if let Some(remote) = configuration.get("remote") {
            let trimmed = remote.trim();
            if !trimmed.is_empty() {
                config.remote = trimmed.to_string();
            }
        }
        
        if let Some(auto_fetch) = configuration.get("auto_fetch") {
            config.auto_fetch = auto_fetch.trim().eq_ignore_ascii_case("true");
        }
        
        if let Some(pattern) = configuration.get("worktree_naming_pattern") {
            config.worktree_naming_pattern = match pattern.trim() {
                "hash" => WorktreeNamingPattern::Hash,
                "branch-hash" => WorktreeNamingPattern::BranchHash,
                _ => WorktreeNamingPattern::Branch,
            };
        }
        
        config
    }
    
    /// Parse config from TOML file contents
    pub fn from_toml(toml_content: &str) -> Result<Self, String> {
        toml::from_str(toml_content).map_err(|e| format!("Failed to parse .zitree.toml: {}", e))
    }
    
    /// Merge repo config over KDL config (repo config takes precedence)
    pub fn merge(&mut self, repo_config: Config) {
        // Only override if the repo config differs from default
        let default = Config::default();
        
        if repo_config.worktree_dir_name != default.worktree_dir_name {
            self.worktree_dir_name = repo_config.worktree_dir_name;
        }
        
        if repo_config.session_prefix.is_some() {
            self.session_prefix = repo_config.session_prefix;
        }
        
        if repo_config.base_branch.is_some() {
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
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
