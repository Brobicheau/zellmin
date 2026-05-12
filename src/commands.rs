use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandAction {
    DiscoverRepo,
    LoadRepoConfig,
    FetchRemote { branch: String },
    CheckBranch { branch: String },
    CreateWorktree { branch: String },
    CreateSession { branch: String, path: PathBuf, session: String },
    ListWorktrees,
    DeleteSession { session: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeLocation {
    pub branch: String,
    pub path: PathBuf,
    pub is_current: bool,
}

pub fn discover_repo() {
    let initial_cwd = get_plugin_ids().initial_cwd;
    run_command_with_env_variables_and_cwd(
        &[
            "git",
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-common-dir",
        ],
        BTreeMap::new(),
        initial_cwd,
        BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_DISCOVER_REPO.to_string())]),
    );
}

pub fn load_repo_config(repo_root: PathBuf) {
    let config_path = repo_root.join(".zitree.toml");
    let config_path_str = config_path.display().to_string();
    run_command_with_env_variables_and_cwd(
        &["cat", &config_path_str],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_LOAD_REPO_CONFIG.to_string())]),
    );
}

pub fn fetch_remote(repo_root: PathBuf, remote: &str, branch: &str) {
    run_command_with_env_variables_and_cwd(
        &["git", "fetch", remote],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_FETCH_REMOTE.to_string()),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
        ]),
    );
}

pub fn check_branch(repo_root: PathBuf, branch: &str) {
    run_command_with_env_variables_and_cwd(
        &["git", "rev-parse", "--verify", &format!("refs/heads/{branch}")],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CHECK_BRANCH.to_string()),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
        ]),
    );
}

pub fn create_worktree(
    repo_root: PathBuf,
    branch: &str,
    worktree_path: &Path,
    base_branch: Option<&str>,
) {
    let worktree_path_string = worktree_path.display().to_string();
    let mut command = vec!["git", "worktree", "add", worktree_path_string.as_str()];
    
    // If branch exists, we'll just add it as a target, otherwise create new branch
    // This is determined by the caller based on check_branch result
    command.push("-b");
    command.push(branch);
    
    if let Some(base) = base_branch {
        command.push(base);
    }

    run_command_with_env_variables_and_cwd(
        &command,
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CREATE_WORKTREE.to_string()),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
        ]),
    );
}

pub fn create_worktree_existing(repo_root: PathBuf, branch: &str, worktree_path: &Path) {
    let worktree_path_string = worktree_path.display().to_string();
    let command = vec!["git", "worktree", "add", worktree_path_string.as_str(), branch];

    run_command_with_env_variables_and_cwd(
        &command,
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CREATE_WORKTREE.to_string()),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
        ]),
    );
}

pub fn list_worktrees(repo_root: PathBuf) {
    run_command_with_env_variables_and_cwd(
        &["git", "worktree", "list", "--porcelain"],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_LIST_WORKTREES.to_string())]),
    );
}

pub fn create_session(branch: &str, worktree_path: &Path, session_name: &str) {
    let worktree_path_string = worktree_path.display().to_string();
    run_command_with_env_variables_and_cwd(
        &["zellij", "attach", "--create-background", session_name],
        BTreeMap::new(),
        worktree_path.to_path_buf(),
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CREATE_SESSION.to_string()),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
            (CONTEXT_PATH.to_string(), worktree_path_string),
            (CONTEXT_SESSION.to_string(), session_name.to_string()),
        ]),
    );
}

pub fn delete_session(repo_root: PathBuf, session_name: &str) {
    run_command_with_env_variables_and_cwd(
        &["zellij", "delete-session", session_name, "--force"],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_DELETE_SESSION.to_string()),
            (CONTEXT_SESSION.to_string(), session_name.to_string()),
        ]),
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
            Some(CommandAction::CreateSession { branch, path, session })
        }
        ACTION_LIST_WORKTREES => Some(CommandAction::ListWorktrees),
        ACTION_DELETE_SESSION => {
            let session = context.get(CONTEXT_SESSION)?.clone();
            Some(CommandAction::DeleteSession { session })
        }
        _ => None,
    }
}

pub fn parse_repo_roots(output: &str) -> Option<(PathBuf, PathBuf)> {
    let mut lines = output.lines().map(str::trim).filter(|line| !line.is_empty());
    let current_worktree_root = PathBuf::from(lines.next()?);
    let git_common_dir = PathBuf::from(lines.next()?);
    let repo_root = git_common_dir.parent()?.to_path_buf();
    Some((current_worktree_root, repo_root))
}

pub fn parse_worktree_locations(
    output: &str,
    current_repo_root: Option<&Path>,
) -> Vec<WorktreeLocation> {
    output
        .split("\n\n")
        .filter_map(|block| parse_worktree_location_block(block, current_repo_root))
        .collect()
}

fn parse_worktree_location_block(
    block: &str,
    current_repo_root: Option<&Path>,
) -> Option<WorktreeLocation> {
    let mut path = None;
    let mut branch = None;

    for line in block.lines() {
        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value.trim()));
        } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
            branch = Some(value.trim().to_string());
        }
    }

    let path = path?;
    let branch = branch?;
    Some(WorktreeLocation {
        is_current: current_repo_root == Some(path.as_path()),
        branch,
        path,
    })
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
    fn parses_current_and_shared_repo_roots_for_linked_worktree() {
        let roots = parse_repo_roots("/tmp/repo/.worktrees/feature\n/tmp/repo/.git\n");

        assert_eq!(
            roots,
            Some((
                PathBuf::from("/tmp/repo/.worktrees/feature"),
                PathBuf::from("/tmp/repo"),
            ))
        );
    }

    #[test]
    fn parses_current_and_shared_repo_roots_for_main_worktree() {
        let roots = parse_repo_roots("/tmp/repo\n/tmp/repo/.git\n");

        assert_eq!(
            roots,
            Some((PathBuf::from("/tmp/repo"), PathBuf::from("/tmp/repo")))
        );
    }

    #[test]
    fn parses_main_and_linked_worktrees_into_locations() {
        let output = "worktree /tmp/repo\nHEAD abc123\nbranch refs/heads/main\n\nworktree /tmp/repo/.worktrees/feature\nHEAD def456\nbranch refs/heads/feature/test\n";

        let worktrees = parse_worktree_locations(output, Some(Path::new("/tmp/repo")));

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].path, PathBuf::from("/tmp/repo"));
        assert!(worktrees[0].is_current);

        assert_eq!(worktrees[1].branch, "feature/test");
        assert_eq!(
            worktrees[1].path,
            PathBuf::from("/tmp/repo/.worktrees/feature")
        );
        assert!(!worktrees[1].is_current);
    }

    #[test]
    fn marks_linked_worktree_as_current_when_started_inside_it() {
        let output = "worktree /tmp/repo\nHEAD abc123\nbranch refs/heads/main\n\nworktree /tmp/repo/.worktrees/feature\nHEAD def456\nbranch refs/heads/feature/test\n";

        let worktrees = parse_worktree_locations(
            output,
            Some(Path::new("/tmp/repo/.worktrees/feature")),
        );

        assert!(!worktrees[0].is_current);
        assert!(worktrees[1].is_current);
    }

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
}
