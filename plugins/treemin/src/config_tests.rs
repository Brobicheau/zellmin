#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.worktree_dir_name, ".worktrees");
        assert_eq!(config.remote, "origin");
        assert_eq!(config.auto_fetch, false);
        assert_eq!(config.worktree_naming_pattern, WorktreeNamingPattern::Branch);
        assert!(config.session_prefix.is_none());
        assert!(config.base_branch.is_none());
    }

    #[test]
    fn test_from_kdl_basic() {
        use std::collections::BTreeMap;
        
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), "trees".to_string());
        kdl.insert("session_prefix".to_string(), "wt".to_string());
        
        let config = Config::from_kdl(kdl);
        assert_eq!(config.worktree_dir_name, "trees");
        assert_eq!(config.session_prefix, Some("wt".to_string()));
    }

    #[test]
    fn test_from_kdl_all_options() {
        use std::collections::BTreeMap;
        
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), ".worktrees".to_string());
        kdl.insert("session_prefix".to_string(), "dev".to_string());
        kdl.insert("base_branch".to_string(), "main".to_string());
        kdl.insert("remote".to_string(), "upstream".to_string());
        kdl.insert("auto_fetch".to_string(), "true".to_string());
        kdl.insert("worktree_naming_pattern".to_string(), "branch-hash".to_string());
        
        let config = Config::from_kdl(kdl);
        assert_eq!(config.worktree_dir_name, ".worktrees");
        assert_eq!(config.session_prefix, Some("dev".to_string()));
        assert_eq!(config.base_branch, Some("main".to_string()));
        assert_eq!(config.remote, "upstream");
        assert_eq!(config.auto_fetch, true);
        assert_eq!(config.worktree_naming_pattern, WorktreeNamingPattern::BranchHash);
    }

    #[test]
    fn test_from_toml() {
        let toml_content = r#"
worktree_dir_name = "trees"
session_prefix = "wt"
base_branch = "develop"
remote = "origin"
auto_fetch = true
worktree_naming_pattern = "hash"
"#;
        
        let config = Config::from_toml(toml_content).unwrap();
        assert_eq!(config.worktree_dir_name, "trees");
        assert_eq!(config.session_prefix, Some("wt".to_string()));
        assert_eq!(config.base_branch, Some("develop".to_string()));
        assert_eq!(config.remote, "origin");
        assert_eq!(config.auto_fetch, true);
        assert_eq!(config.worktree_naming_pattern, WorktreeNamingPattern::Hash);
    }

    #[test]
    fn test_merge_precedence() {
        use std::collections::BTreeMap;
        
        // Start with KDL config
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), "kdl-trees".to_string());
        kdl.insert("session_prefix".to_string(), "kdl".to_string());
        
        let mut config = Config::from_kdl(kdl);
        
        // Repo config overrides
        let toml_content = r#"
worktree_dir_name = "repo-trees"
base_branch = "main"
"#;
        
        let repo_config = Config::from_toml(toml_content).unwrap();
        config.merge(repo_config);
        
        // Repo config should override worktree_dir_name
        assert_eq!(config.worktree_dir_name, "repo-trees");
        // But KDL session_prefix should remain since repo didn't set it
        assert_eq!(config.session_prefix, Some("kdl".to_string()));
        // Repo config should add base_branch
        assert_eq!(config.base_branch, Some("main".to_string()));
    }

    #[test]
    fn test_empty_strings_ignored() {
        use std::collections::BTreeMap;
        
        let mut kdl = BTreeMap::new();
        kdl.insert("worktree_dir_name".to_string(), "  ".to_string());
        kdl.insert("session_prefix".to_string(), "".to_string());
        
        let config = Config::from_kdl(kdl);
        // Empty/whitespace strings should be ignored, using defaults
        assert_eq!(config.worktree_dir_name, ".worktrees");
        assert!(config.session_prefix.is_none());
    }
}
