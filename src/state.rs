use std::collections::BTreeMap;
use std::path::PathBuf;

use zellij_tile::prelude::*;

use crate::config::Config;
use crate::naming;
use crate::ui;
use crate::validation::{is_branch_char, validate_branch_name};

const CONTEXT_ACTION: &str = "action";
const ACTION_DISCOVER_REPO: &str = "discover-repo";
const ACTION_CHECK_BRANCH: &str = "check-branch";
const ACTION_CREATE_WORKTREE: &str = "create-worktree";

#[derive(Default)]
pub struct State {
    config: Config,
    permissions_granted: bool,
    repo_root: Option<PathBuf>,
    repo_name: Option<String>,
    branch_input: String,
    status: Status,
    sessions: Vec<String>,
}

#[derive(Default)]
pub enum Status {
    #[default]
    Loading,
    Ready,
    Busy(String),
    Error(String),
    Success(String),
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from(configuration);
        set_selectable(true);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
            EventType::Key,
            EventType::SessionUpdate,
        ]);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        self.status = Status::Loading;
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(status) => {
                self.permissions_granted = matches!(status, PermissionStatus::Granted);
                if self.permissions_granted {
                    self.refresh_sessions();
                    self.discover_repo();
                } else {
                    self.status = Status::Error(
                        "Permission request was denied. Reload the plugin and grant access."
                            .to_string(),
                    );
                }
                true
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                self.handle_run_command_result(exit_code, stdout, stderr, context);
                true
            }
            Event::Key(key) => self.handle_key(key),
            Event::SessionUpdate(live_sessions, _) => {
                self.sessions = live_sessions.into_iter().map(|session| session.name).collect();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        ui::render(
            &self.status,
            self.repo_root.as_deref(),
            &self.config.worktree_dir_name,
            &self.branch_input,
            cols,
        );
    }
}

impl State {
    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        if !self.permissions_granted {
            return false;
        }

        match key.bare_key {
            BareKey::Enter => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.begin_create_worktree();
                true
            }
            BareKey::Backspace => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.branch_input.pop();
                self.status = Status::Ready;
                true
            }
            BareKey::Esc => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.branch_input.clear();
                self.status = Status::Ready;
                true
            }
            BareKey::Char(character) => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                if key.has_no_modifiers() && is_branch_char(character) {
                    self.branch_input.push(character);
                    self.status = Status::Ready;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn begin_create_worktree(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        let branch = self.branch_input.trim();
        if branch.is_empty() {
            self.status = Status::Error("Enter a branch name first.".to_string());
            return;
        }
        if let Err(message) = validate_branch_name(branch) {
            self.status = Status::Error(message);
            return;
        }

        let context = BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CHECK_BRANCH.to_string()),
            ("branch".to_string(), branch.to_string()),
        ]);
        self.status = Status::Busy(format!("Checking branch `{branch}`..."));
        run_command_with_env_variables_and_cwd(
            &["git", "rev-parse", "--verify", &format!("refs/heads/{branch}")],
            BTreeMap::new(),
            repo_root,
            context,
        );
    }

    fn handle_run_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        context: BTreeMap<String, String>,
    ) {
        let Some(action) = context.get(CONTEXT_ACTION).map(String::as_str) else {
            return;
        };

        match action {
            ACTION_DISCOVER_REPO => {
                if exit_code == Some(0) {
                    let output = String::from_utf8_lossy(&stdout).trim().to_string();
                    if output.is_empty() {
                        self.status =
                            Status::Error("Could not determine git repository root.".to_string());
                        return;
                    }
                    let repo_root = PathBuf::from(&output);
                    self.repo_name = repo_root
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_string());
                    self.repo_root = Some(repo_root);
                    self.status = Status::Ready;
                } else {
                    self.status = Status::Error(command_error(
                        "Failed to discover repository root.",
                        &stderr,
                    ));
                }
            }
            ACTION_CHECK_BRANCH => {
                let Some(branch) = context.get("branch") else {
                    self.status = Status::Error("Branch context was missing.".to_string());
                    return;
                };
                self.create_worktree(branch, exit_code == Some(0));
            }
            ACTION_CREATE_WORKTREE => {
                let Some(branch) = context.get("branch") else {
                    self.status = Status::Error("Branch context was missing.".to_string());
                    return;
                };

                if exit_code == Some(0) {
                    let worktree_path = self.worktree_path(branch);
                    let session_name = self.session_name(branch);
                    self.status = Status::Success(format!(
                        "Created worktree `{}`. Switching to session `{session_name}`...",
                        worktree_path.display()
                    ));
                    switch_session_with_cwd(Some(&session_name), Some(worktree_path));
                } else {
                    self.status = Status::Error(command_error("Failed to create worktree.", &stderr));
                }
            }
            _ => {}
        }
    }

    fn create_worktree(&mut self, branch: &str, branch_exists: bool) {
        let worktree_path = self.worktree_path(branch);

        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        let worktree_path_string = worktree_path.display().to_string();
        let mut command = vec!["git", "worktree", "add", worktree_path_string.as_str()];
        let existing_branch_target;
        if branch_exists {
            existing_branch_target = branch.to_string();
            command.push(existing_branch_target.as_str());
        } else {
            command.push("-b");
            command.push(branch);
        }

        let context = BTreeMap::from([
            (CONTEXT_ACTION.to_string(), ACTION_CREATE_WORKTREE.to_string()),
            ("branch".to_string(), branch.to_string()),
        ]);

        self.status = Status::Busy(format!("Creating worktree `{}`...", worktree_path.display()));
        run_command_with_env_variables_and_cwd(&command, BTreeMap::new(), repo_root, context);
    }

    fn discover_repo(&mut self) {
        let initial_cwd = get_plugin_ids().initial_cwd;
        self.status = Status::Busy("Discovering repository root...".to_string());
        run_command_with_env_variables_and_cwd(
            &["git", "rev-parse", "--show-toplevel"],
            BTreeMap::new(),
            initial_cwd,
            BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_DISCOVER_REPO.to_string())]),
        );
    }

    fn refresh_sessions(&mut self) {
        if let Ok(snapshot) = get_session_list() {
            self.sessions = snapshot
                .live_sessions
                .into_iter()
                .map(|session| session.name)
                .collect();
        }
    }

    fn worktree_path(&self, branch: &str) -> PathBuf {
        let repo_root = self
            .repo_root
            .as_deref()
            .expect("repo root should be available before creating worktrees");
        naming::worktree_path(repo_root, &self.config.worktree_dir_name, branch)
    }

    fn session_name(&self, branch: &str) -> String {
        naming::session_name(self.repo_name.as_deref(), branch)
    }
}

fn command_error(prefix: &str, stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {stderr}")
    }
}
