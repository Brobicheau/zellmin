use std::collections::BTreeMap;
use std::path::PathBuf;

use zellij_tile::prelude::*;

pub mod git;
pub mod zellij;

pub use git::WorktreeLocation;

const CONTEXT_ACTION: &str = "action";
const CONTEXT_BRANCH: &str = "branch";
const CONTEXT_PATH: &str = "path";
const CONTEXT_SESSION: &str = "session";

const ACTION_DISCOVER_REPO: &str = "discover-repo";
const ACTION_LOAD_REPO_CONFIG: &str = "load-repo-config";
const ACTION_FETCH_REMOTE: &str = "fetch-remote";
const ACTION_CHECK_BRANCH: &str = "check-branch";
const ACTION_CREATE_WORKTREE: &str = "create-worktree";
const ACTION_CREATE_SESSION: &str = "create-session";
const ACTION_LIST_WORKTREES: &str = "list-worktrees";
const ACTION_DELETE_SESSION: &str = "delete-session";
const ACTION_DELETE_WORKTREE: &str = "delete-worktree";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandAction {
    DiscoverRepo,
    LoadRepoConfig,
    FetchRemote { branch: String },
    CheckBranch { branch: String },
    CreateWorktree { branch: String },
    CreateSession {
        branch: String,
        path: PathBuf,
        session: String,
    },
    ListWorktrees,
    DeleteSession { session: String },
    DeleteWorktree { path: PathBuf },
}

pub fn load_repo_config(repo_root: PathBuf) {
    let config_path = repo_root.join(".treemin.toml");
    let config_path_str = config_path.display().to_string();
    run_command_with_env_variables_and_cwd(
        &["cat", &config_path_str],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_LOAD_REPO_CONFIG.to_string())]),
    );
}

pub fn parse_action(context: &BTreeMap<String, String>) -> Option<CommandAction> {
    let action = context.get(CONTEXT_ACTION)?;

    match action.as_str() {
        ACTION_DISCOVER_REPO => Some(CommandAction::DiscoverRepo),
        ACTION_LOAD_REPO_CONFIG => Some(CommandAction::LoadRepoConfig),
        ACTION_FETCH_REMOTE => {
            let branch = context.get(CONTEXT_BRANCH)?.clone();
            Some(CommandAction::FetchRemote { branch })
        }
        ACTION_CHECK_BRANCH => {
            let branch = context.get(CONTEXT_BRANCH)?.clone();
            Some(CommandAction::CheckBranch { branch })
        }
        ACTION_CREATE_WORKTREE => {
            let branch = context.get(CONTEXT_BRANCH)?.clone();
            Some(CommandAction::CreateWorktree { branch })
        }
        ACTION_CREATE_SESSION => {
            let branch = context.get(CONTEXT_BRANCH)?.clone();
            let path = context.get(CONTEXT_PATH).map(PathBuf::from)?;
            let session = context.get(CONTEXT_SESSION)?.clone();
            Some(CommandAction::CreateSession {
                branch,
                path,
                session,
            })
        }
        ACTION_LIST_WORKTREES => Some(CommandAction::ListWorktrees),
        ACTION_DELETE_SESSION => {
            let session = context.get(CONTEXT_SESSION)?.clone();
            Some(CommandAction::DeleteSession { session })
        }
        ACTION_DELETE_WORKTREE => {
            let path = context.get(CONTEXT_PATH).map(PathBuf::from)?;
            Some(CommandAction::DeleteWorktree { path })
        }
        _ => None,
    }
}

pub fn command_error(prefix: &str, stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {stderr}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_discover_repo_action() {
        let context = BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_DISCOVER_REPO.to_string())]);

        assert_eq!(parse_action(&context), Some(CommandAction::DiscoverRepo));
    }

    #[test]
    fn parses_fetch_remote_action_with_branch() {
        let context = BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_FETCH_REMOTE.to_string()),
            (CONTEXT_BRANCH.to_string(), "feature/test".to_string()),
        ]);

        assert_eq!(
            parse_action(&context),
            Some(CommandAction::FetchRemote {
                branch: "feature/test".to_string()
            })
        );
    }

    #[test]
    fn parses_create_session_action_with_all_context() {
        let context = BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CREATE_SESSION.to_string()),
            (CONTEXT_BRANCH.to_string(), "main".to_string()),
            (CONTEXT_PATH.to_string(), "/tmp/repo".to_string()),
            (CONTEXT_SESSION.to_string(), "repo-main".to_string()),
        ]);

        assert_eq!(
            parse_action(&context),
            Some(CommandAction::CreateSession {
                branch: "main".to_string(),
                path: PathBuf::from("/tmp/repo"),
                session: "repo-main".to_string()
            })
        );
    }

    #[test]
    fn parses_delete_worktree_action_with_path() {
        let context = BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_DELETE_WORKTREE.to_string()),
            (CONTEXT_PATH.to_string(), "/tmp/repo/.worktrees/feature".to_string()),
        ]);

        assert_eq!(
            parse_action(&context),
            Some(CommandAction::DeleteWorktree {
                path: PathBuf::from("/tmp/repo/.worktrees/feature")
            })
        );
    }
}
