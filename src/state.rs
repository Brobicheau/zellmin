use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use zellij_tile::prelude::*;

use crate::config::Config;
use crate::naming;
use crate::ui;
use crate::validation::{is_branch_char, validate_branch_name};

const CONTEXT_ACTION: &str = "action";
const ACTION_DISCOVER_REPO: &str = "discover-repo";
const ACTION_LOAD_REPO_CONFIG: &str = "load-repo-config";
const ACTION_FETCH_REMOTE: &str = "fetch-remote";
const ACTION_CHECK_BRANCH: &str = "check-branch";
const ACTION_CREATE_WORKTREE: &str = "create-worktree";
const ACTION_LIST_WORKTREES: &str = "list-worktrees";

#[derive(Default)]
pub struct State {
    config: Config,
    kdl_config: Config,
    permissions_granted: bool,
    repo_root: Option<PathBuf>,
    repo_name: Option<String>,
    branch_input: String,
    known_worktrees: Vec<WorktreeLocation>,
    worktree_sessions: Vec<WorktreeSessionEntry>,
    live_session_names: Vec<String>,
    selected_index: usize,
    status: Status,
    config_loaded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeSessionEntry {
    pub branch: String,
    pub path: Option<PathBuf>,
    pub session_name: String,
    pub live_session_name: Option<String>,
    pub has_live_session: bool,
    pub is_current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorktreeLocation {
    branch: String,
    path: PathBuf,
    is_current: bool,
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
        self.kdl_config = Config::from_kdl(configuration);
        self.config = self.kdl_config.clone();
        self.config_loaded = false;
        set_selectable(true);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
            EventType::SessionUpdate,
            EventType::Key,
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
            Event::SessionUpdate(live_sessions, _) => {
                self.live_session_names = live_sessions.into_iter().map(|session| session.name).collect();
                self.rebuild_worktree_sessions();
                true
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        ui::render(
            &self.status,
            self.repo_root.as_deref(),
            &self.config,
            &self.branch_input,
            &self.worktree_sessions,
            self.selected_index,
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
                self.begin_primary_action();
                true
            }
            BareKey::Up => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.select_previous_worktree_session();
                true
            }
            BareKey::Down => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.select_next_worktree_session();
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

    fn begin_primary_action(&mut self) {
        if self.branch_input.trim().is_empty() {
            self.switch_selected_worktree_session();
        } else {
            self.begin_create_worktree();
        }
    }

    fn begin_create_worktree(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        if !self.config_loaded {
            self.status = Status::Error("Configuration is still loading.".to_string());
            return;
        }

        let branch = self.branch_input.trim().to_string();
        if branch.is_empty() {
            self.status = Status::Error("Enter a branch name first.".to_string());
            return;
        }
        if let Err(message) = validate_branch_name(&branch) {
            self.status = Status::Error(message);
            return;
        }

        // If auto_fetch is enabled, fetch first
        if self.config.auto_fetch {
            let context = BTreeMap::from([
                (CONTEXT_ACTION.to_string(), ACTION_FETCH_REMOTE.to_string()),
                ("branch".to_string(), branch.clone()),
            ]);
            self.status = Status::Busy(format!("Fetching from remote `{}`...", self.config.remote));
            run_command_with_env_variables_and_cwd(
                &["git", "fetch", &self.config.remote],
                BTreeMap::new(),
                repo_root,
                context,
            );
        } else {
            self.check_branch(&branch);
        }
    }
    
    fn check_branch(&mut self, branch: &str) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

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
                    self.repo_root = Some(repo_root.clone());
                    
                    // Try to load repo config
                    self.load_repo_config(repo_root);
                } else {
                    self.status = Status::Error(command_error(
                        "Failed to discover repository root.",
                        &stderr,
                    ));
                }
            }
            ACTION_LOAD_REPO_CONFIG => {
                if exit_code == Some(0) {
                    let toml_content = String::from_utf8_lossy(&stdout).to_string();
                    if !toml_content.trim().is_empty() {
                        match Config::from_toml(&toml_content) {
                            Ok(repo_config) => {
                                self.config = self.kdl_config.clone();
                                self.config.merge(repo_config);
                            }
                            Err(err) => {
                                self.status = Status::Error(err);
                                self.config_loaded = true;
                                return;
                            }
                        }
                    } else {
                        self.config = self.kdl_config.clone();
                    }
                } else {
                    // Config file doesn't exist, use KDL config
                    self.config = self.kdl_config.clone();
                }
                self.config_loaded = true;
                self.refresh_worktree_sessions();
            }
            ACTION_FETCH_REMOTE => {
                if exit_code == Some(0) {
                    let Some(branch) = context.get("branch") else {
                        self.status = Status::Error("Branch context was missing.".to_string());
                        return;
                    };
                    // After fetch, check if branch exists
                    self.check_branch(branch);
                } else {
                    self.status = Status::Error(command_error(
                        "Failed to fetch from remote.",
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
                    self.add_or_select_worktree_session(branch, &worktree_path, &session_name);
                    self.status = Status::Success(format!(
                        "Created worktree `{}`. Switching to session `{session_name}`...",
                        worktree_path.display()
                    ));
                    switch_session_with_cwd(Some(&session_name), Some(worktree_path));
                } else {
                    self.status = Status::Error(command_error("Failed to create worktree.", &stderr));
                }
            }
            ACTION_LIST_WORKTREES => {
                if exit_code == Some(0) {
                    let previous_selection = self.selected_session_key();
                    self.known_worktrees = parse_worktree_locations(
                        &String::from_utf8_lossy(&stdout),
                        self.repo_root.as_deref(),
                    );
                    self.rebuild_worktree_sessions_with_selection(
                        previous_selection.as_ref().map(|value| value.as_str()),
                    );
                    self.status = Status::Ready;
                } else {
                    self.status = Status::Error(command_error("Failed to load worktree sessions.", &stderr));
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
            // If base_branch is configured, use it as the starting point
            if let Some(base_branch) = &self.config.base_branch {
                command.push(base_branch.as_str());
            }
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
    
    fn load_repo_config(&mut self, repo_root: PathBuf) {
        let config_path = repo_root.join(".zitree.toml");
        let config_path_str = config_path.display().to_string();
        self.status = Status::Busy("Loading repository configuration...".to_string());
        run_command_with_env_variables_and_cwd(
            &["cat", &config_path_str],
            BTreeMap::new(),
            repo_root,
            BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_LOAD_REPO_CONFIG.to_string())]),
        );
    }

    fn refresh_worktree_sessions(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        self.status = Status::Busy("Loading worktree sessions...".to_string());
        run_command_with_env_variables_and_cwd(
            &["git", "worktree", "list", "--porcelain"],
            BTreeMap::new(),
            repo_root,
            BTreeMap::from([(CONTEXT_ACTION.to_string(), ACTION_LIST_WORKTREES.to_string())]),
        );
    }

    fn select_previous_worktree_session(&mut self) {
        if self.worktree_sessions.is_empty() {
            self.status = Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        }

        self.selected_index = self.selected_index.saturating_sub(1);
        self.status = Status::Ready;
    }

    fn select_next_worktree_session(&mut self) {
        if self.worktree_sessions.is_empty() {
            self.status = Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        }

        self.selected_index = (self.selected_index + 1).min(self.worktree_sessions.len() - 1);
        self.status = Status::Ready;
    }

    fn switch_selected_worktree_session(&mut self) {
        let Some(entry) = self.worktree_sessions.get(self.selected_index) else {
            self.status = Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        };

        if !entry.has_live_session {
            self.status = Status::Error(format!(
                "No live Zellij session matching `{}` was found for `{}`.",
                entry.session_name, entry.branch
            ));
            return;
        }

        let Some(live_session_name) = entry.live_session_name.as_deref() else {
            self.status = Status::Error(format!(
                "No live Zellij session match was found for `{}`.",
                entry.branch
            ));
            return;
        };

        self.status = Status::Success(format!(
            "Switching to live session `{}`...",
            live_session_name,
        ));
        switch_session(Some(live_session_name));
    }

    fn add_or_select_worktree_session(&mut self, branch: &str, path: &Path, session_name: &str) {
        if let Some(index) = self
            .worktree_sessions
            .iter()
            .position(|entry| entry.live_session_name.as_deref() == Some(session_name))
        {
            self.selected_index = index;
            return;
        }

        self.worktree_sessions.push(WorktreeSessionEntry {
            branch: branch.to_string(),
            path: Some(path.to_path_buf()),
            session_name: session_name.to_string(),
            live_session_name: Some(session_name.to_string()),
            has_live_session: true,
            is_current: false,
        });
        self.selected_index = self.worktree_sessions.len() - 1;
    }

    fn rebuild_worktree_sessions(&mut self) {
        let previous_selection = self.selected_session_key();
        self.rebuild_worktree_sessions_with_selection(
            previous_selection.as_ref().map(|value| value.as_str()),
        );
    }

    fn rebuild_worktree_sessions_with_selection(&mut self, previous_selection: Option<&str>) {
        self.worktree_sessions = build_worktree_sessions(
            self.repo_name.as_deref(),
            &self.config,
            &self.known_worktrees,
            &self.live_session_names,
        );
        self.selected_index = selected_index_for_sessions(&self.worktree_sessions, previous_selection);
    }

    fn selected_session_key(&self) -> Option<String> {
        self.worktree_sessions
            .get(self.selected_index)
            .map(session_selection_key)
    }

    fn worktree_path(&self, branch: &str) -> PathBuf {
        let repo_root = self
            .repo_root
            .as_deref()
            .expect("repo root should be available before creating worktrees");
        naming::worktree_path(repo_root, &self.config, branch)
    }

    fn session_name(&self, branch: &str) -> String {
        naming::session_name(self.repo_name.as_deref(), branch, &self.config)
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

fn parse_worktree_locations(
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

fn build_worktree_sessions(
    repo_name: Option<&str>,
    config: &Config,
    known_worktrees: &[WorktreeLocation],
    live_session_names: &[String],
) -> Vec<WorktreeSessionEntry> {
    let Some(repo_name) = repo_name else {
        return Vec::new();
    };

    let mut sessions = Vec::new();
    let mut matched_live_sessions = Vec::new();

    for worktree in known_worktrees {
        let candidates = worktree_session_name_candidates(Some(repo_name), &worktree.branch, config);
        if let Some(live_session_name) = live_session_names.iter().find(|session_name| {
            candidates.iter().any(|candidate| candidate == *session_name)
        }) {
            matched_live_sessions.push(live_session_name.clone());
            sessions.push(WorktreeSessionEntry {
                branch: worktree.branch.clone(),
                path: Some(worktree.path.clone()),
                session_name: naming::session_name(Some(repo_name), &worktree.branch, config),
                live_session_name: Some(live_session_name.clone()),
                has_live_session: true,
                is_current: worktree.is_current,
            });
        }
    }

    for live_session_name in live_session_names {
        if matched_live_sessions.iter().any(|matched| matched == live_session_name) {
            continue;
        }
        if !session_belongs_to_repo(live_session_name, repo_name, config) {
            continue;
        }

        sessions.push(WorktreeSessionEntry {
            branch: orphan_branch_label(live_session_name, repo_name, config),
            path: None,
            session_name: live_session_name.clone(),
            live_session_name: Some(live_session_name.clone()),
            has_live_session: true,
            is_current: false,
        });
    }

    sessions.sort_by(|left, right| left.branch.cmp(&right.branch));
    sessions
}

fn worktree_session_name_candidates(
    repo_name: Option<&str>,
    branch: &str,
    config: &Config,
) -> Vec<String> {
    naming::session_name_candidates(repo_name, branch, config)
}

fn selected_index_for_sessions(
    sessions: &[WorktreeSessionEntry],
    previous_selection: Option<&str>,
) -> usize {
    if sessions.is_empty() {
        return 0;
    }

    if let Some(previous_selection) = previous_selection {
        if let Some(index) = sessions
            .iter()
            .position(|entry| session_selection_key(entry) == previous_selection)
        {
            return index;
        }
    }

    sessions.iter().position(|entry| entry.is_current).unwrap_or(0)
}

fn session_selection_key(entry: &WorktreeSessionEntry) -> String {
    entry
        .live_session_name
        .clone()
        .unwrap_or_else(|| entry.session_name.clone())
}

fn session_belongs_to_repo(live_session_name: &str, repo_name: &str, config: &Config) -> bool {
    session_segments_for_repo(live_session_name, repo_name, config).is_some()
}

fn orphan_branch_label(
    live_session_name: &str,
    repo_name: &str,
    config: &Config,
 ) -> String {
    orphan_branch_segment(live_session_name, repo_name, config)
        .map(|branch| branch.replace('-', "/"))
        .unwrap_or_else(|| live_session_name.to_string())
}

fn orphan_branch_segment(
    live_session_name: &str,
    repo_name: &str,
    config: &Config,
 ) -> Option<String> {
    let (_repo_segment, branch_segment) = session_segments_for_repo(live_session_name, repo_name, config)?;
    Some(branch_segment)
}

fn session_segments_for_repo(
    live_session_name: &str,
    repo_name: &str,
    config: &Config,
) -> Option<(String, String)> {
    let hash_start = live_session_name.rfind('-')?;
    let without_hash = &live_session_name[..hash_start];
    let prefix = config
        .session_prefix
        .as_deref()
        .map(naming::sanitize_session_segment);
    let without_prefix = if let Some(prefix) = prefix.as_deref() {
        without_hash.strip_prefix(prefix)?.strip_prefix('-')?
    } else {
        without_hash
    };

    let mut best_match = None;
    for (index, character) in without_prefix.char_indices() {
        if character != '-' {
            continue;
        }

        let repo_segment = &without_prefix[..index];
        let branch_segment = &without_prefix[index + 1..];
        if branch_segment.is_empty() || !session_segment_matches_repo(repo_segment, repo_name) {
            continue;
        }

        best_match = Some((repo_segment.to_string(), branch_segment.to_string()));
    }

    best_match
}

fn session_segment_matches_repo(session_repo_segment: &str, repo_name: &str) -> bool {
    let sanitized_repo = naming::sanitize_session_segment(repo_name);
    if session_repo_segment == sanitized_repo {
        return true;
    }

    truncate_matches_repo(session_repo_segment, &sanitized_repo)
}

fn truncate_matches_repo(session_repo_segment: &str, sanitized_repo: &str) -> bool {
    if session_repo_segment.is_empty() {
        return false;
    }
    naming::truncate_session_segment(sanitized_repo, session_repo_segment.len()) == session_repo_segment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_main_and_linked_worktrees_into_locations() {
        let output = "worktree /tmp/repo\nHEAD abc123\nbranch refs/heads/main\n\nworktree /tmp/repo/.worktrees/feature\nHEAD def456\nbranch refs/heads/feature/test\n";

        let worktrees = parse_worktree_locations(
            output,
            Some(Path::new("/tmp/repo")),
        );

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].path, PathBuf::from("/tmp/repo"));
        assert!(worktrees[0].is_current);

        assert_eq!(worktrees[1].branch, "feature/test");
        assert_eq!(worktrees[1].path, PathBuf::from("/tmp/repo/.worktrees/feature"));
        assert!(!worktrees[1].is_current);
    }

    #[test]
    fn prefers_previous_selection_when_sessions_refresh() {
        let sessions = vec![
            WorktreeSessionEntry {
                branch: "main".to_string(),
                path: Some(PathBuf::from("/tmp/repo")),
                session_name: "repo-main-17c9aaa7".to_string(),
                live_session_name: Some("repo-main-17c9aaa7".to_string()),
                has_live_session: true,
                is_current: true,
            },
            WorktreeSessionEntry {
                branch: "feature".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo-feature-d0b50b87".to_string(),
                live_session_name: Some("repo-feature-d0b50b87".to_string()),
                has_live_session: true,
                is_current: false,
            },
        ];

        let selected_index = selected_index_for_sessions(&sessions, Some("repo-feature-d0b50b87"));

        assert_eq!(selected_index, 1);
    }

    #[test]
    fn builds_sessions_from_live_sessions_and_known_worktrees() {
        let config = Config::default();
        let worktrees = vec![
            WorktreeLocation {
                branch: "main".to_string(),
                path: PathBuf::from("/tmp/repo"),
                is_current: true,
            },
            WorktreeLocation {
                branch: "feature/test".to_string(),
                path: PathBuf::from("/tmp/repo/.worktrees/feature"),
                is_current: false,
            },
        ];
        let sessions = build_worktree_sessions(
            Some("repo"),
            &config,
            &worktrees,
            &[
                "repo-main-17c9aaa7".to_string(),
                "repo-feature-test-727724f6".to_string(),
            ],
        );

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].branch, "main");
        assert_eq!(sessions[0].path, Some(PathBuf::from("/tmp/repo")));
        assert_eq!(sessions[1].branch, "feature/test");
        assert_eq!(sessions[1].path, Some(PathBuf::from("/tmp/repo/.worktrees/feature")));
    }

    #[test]
    fn keeps_live_session_for_repo_when_worktree_is_missing() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some("repo"),
            &config,
            &[],
            &["repo-feature-test-727724f6".to_string()],
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].branch, "feature/test");
        assert_eq!(sessions[0].path, None);
        assert_eq!(sessions[0].live_session_name.as_deref(), Some("repo-feature-test-727724f6"));
    }

    #[test]
    fn ignores_live_session_for_different_repo() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some("repo"),
            &config,
            &[],
            &["other-main-17c9aaa7".to_string()],
        );

        assert!(sessions.is_empty());
    }

    #[test]
    fn matches_truncated_repo_segment_to_repo_name() {
        let config = Config::default();
        let session_name = "wo-4f36e7-feature-727724f6";

        assert!(session_belongs_to_repo(session_name, "workspace", &config));
    }

    #[test]
    fn matches_repo_names_with_dashes() {
        let config = Config::default();
        let session_name = "my-repo-feature-test-727724f6";

        assert!(session_belongs_to_repo(session_name, "my-repo", &config));
        assert_eq!(orphan_branch_segment(session_name, "my-repo", &config).as_deref(), Some("feature-test"));
    }
}
