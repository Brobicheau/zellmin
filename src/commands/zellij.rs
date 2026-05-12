use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use super::{
    ACTION_CREATE_SESSION, ACTION_DELETE_SESSION, CONTEXT_ACTION, CONTEXT_BRANCH, CONTEXT_PATH,
    CONTEXT_SESSION,
};

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
