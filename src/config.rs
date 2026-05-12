use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Config {
    pub worktree_dir_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worktree_dir_name: ".worktrees".to_string(),
        }
    }
}

impl Config {
    pub fn from(configuration: BTreeMap<String, String>) -> Self {
        let mut config = Self::default();
        if let Some(worktree_dir_name) = configuration.get("worktree_dir_name") {
            let trimmed = worktree_dir_name.trim();
            if !trimmed.is_empty() {
                config.worktree_dir_name = trimmed.to_string();
            }
        }
        config
    }
}
