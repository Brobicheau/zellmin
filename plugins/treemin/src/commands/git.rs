use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use super::{
    ACTION_CHECK_BRANCH, ACTION_CREATE_WORKTREE, ACTION_DELETE_WORKTREE, ACTION_DISCOVER_REPO,
    ACTION_FETCH_REMOTE, ACTION_LIST_WORKTREES, CONTEXT_ACTION, CONTEXT_BRANCH, CONTEXT_PATH,
};

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
            (
                CONTEXT_ACTION.to_string(),
                ACTION_CREATE_WORKTREE.to_string(),
            ),
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
            (
                CONTEXT_ACTION.to_string(),
                ACTION_CREATE_WORKTREE.to_string(),
            ),
            (CONTEXT_BRANCH.to_string(), branch.to_string()),
        ]),
    );
}

pub fn list_worktrees(repo_root: PathBuf) {
    run_command_with_env_variables_and_cwd(
        &["git", "worktree", "list", "--porcelain"],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([(
            CONTEXT_ACTION.to_string(),
            ACTION_LIST_WORKTREES.to_string(),
        )]),
    );
}

pub fn delete_worktree(repo_root: PathBuf, worktree_path: &Path) {
    let worktree_path_string = worktree_path.display().to_string();
    run_command_with_env_variables_and_cwd(
        &["git", "worktree", "remove", "--force", &worktree_path_string],
        BTreeMap::new(),
        repo_root,
        BTreeMap::from([
            (
                CONTEXT_ACTION.to_string(),
                ACTION_DELETE_WORKTREE.to_string(),
            ),
            (CONTEXT_PATH.to_string(), worktree_path_string.clone()),
        ]),
    );
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
}
