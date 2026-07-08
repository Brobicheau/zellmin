use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use zellij_tile::prelude::*;

use crate::commands::{self, git, zellij, CommandAction, WorktreeLocation};
use crate::config::Config;
use crate::naming;
use crate::storage::treemin_registry;
use crate::ui;
use crate::validation::{is_branch_char, validate_branch_name};

#[derive(Default)]
pub struct State {
    config: Config,
    kdl_config: Config,
    permissions_granted: bool,
    repo_root: Option<PathBuf>,
    current_worktree_root: Option<PathBuf>,
    repo_name: Option<String>,
    branch_input: String,
    known_worktrees: Vec<WorktreeLocation>,
    worktree_sessions: Vec<WorktreeSessionEntry>,
    live_session_names: Vec<String>,
    selected_index: Option<usize>,
    status: Status,
    config_loaded: bool,
    show_help: bool,
    pending_delete: Option<PendingDelete>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingDelete {
    session_name: Option<String>,
    worktree_path: Option<PathBuf>,
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
        rename_plugin_pane(get_plugin_ids().plugin_id, "treemin");
        set_selectable(true);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
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
                    self.refresh_session_list();
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
            _ => false,
        }
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        ui::render(
            &self.status,
            self.repo_root.as_deref(),
            self.repo_name.as_ref(),
            &self.config,
            &self.branch_input,
            &self.worktree_sessions,
            self.selected_index,
            self.show_help,
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
            BareKey::Char('d') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                self.begin_delete_action();
                true
            }
            BareKey::Char('h') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.show_help = !self.show_help;
                true
            }
            BareKey::Esc => {
                if matches!(self.status, Status::Busy(_)) {
                    return false;
                }
                if self.branch_input.is_empty() {
                    hide_self();
                    false
                } else {
                    self.branch_input.clear();
                    self.status = Status::Ready;
                    true
                }
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
        self.pending_delete = None;
        if self.branch_input.trim().is_empty() {
            self.switch_selected_worktree_session();
        } else {
            self.begin_create_worktree();
        }
    }

    fn begin_delete_action(&mut self) {
        if !self.branch_input.is_empty() {
            self.branch_input.clear();
            self.status = Status::Ready;
            return;
        }

        self.delete_selected_worktree_session();
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
            self.status = Status::Busy(format!("Fetching from remote `{}`...", self.config.remote));
            git::fetch_remote(repo_root, &self.config.remote, &branch);
        } else {
            self.check_branch(&branch);
        }
    }

    fn check_branch(&mut self, branch: &str) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        self.status = Status::Busy(format!("Checking branch `{branch}`..."));
        git::check_branch(repo_root, branch);
    }

    fn handle_run_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        context: BTreeMap<String, String>,
    ) {
        let Some(action) = commands::parse_action(&context) else {
            return;
        };

        match action {
            CommandAction::DiscoverRepo => {
                if exit_code == Some(0) {
                    let output = String::from_utf8_lossy(&stdout);
                    let Some((current_worktree_root, repo_root)) = git::parse_repo_roots(&output)
                    else {
                        self.status =
                            Status::Error("Could not determine git repository root.".to_string());
                        return;
                    };
                    self.repo_name = repo_root
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_string());
                    self.current_worktree_root = Some(current_worktree_root);
                    self.repo_root = Some(repo_root.clone());

                    // Try to load repo config
                    self.load_repo_config(repo_root);
                } else {
                    self.status = Status::Error(commands::command_error(
                        "Failed to discover repository root.",
                        &stderr,
                    ));
                }
            }
            CommandAction::LoadRepoConfig => {
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
            CommandAction::FetchRemote { branch } => {
                if self.pending_delete.is_some() {
                    return;
                }
                if exit_code == Some(0) {
                    // After fetch, check if branch exists
                    self.check_branch(&branch);
                } else {
                    self.status = Status::Error(commands::command_error(
                        "Failed to fetch from remote.",
                        &stderr,
                    ));
                }
            }
            CommandAction::CheckBranch { branch } => {
                if self.pending_delete.is_some() {
                    return;
                }
                self.create_worktree(&branch, exit_code == Some(0));
            }
            CommandAction::CreateWorktree { branch } => {
                if self.pending_delete.is_some() {
                    return;
                }
                if exit_code == Some(0) {
                    self.create_or_switch_worktree_session(&branch);
                } else {
                    self.status = Status::Error(commands::command_error(
                        "Failed to create worktree.",
                        &stderr,
                    ));
                }
            }
            CommandAction::CreateSession {
                branch,
                path: worktree_path,
                session: session_name,
            } => {
                if self.pending_delete.is_some() {
                    self.status = Status::Ready;
                    self.rebuild_worktree_sessions();
                    return;
                }

                if exit_code == Some(0) {
                    if !self
                        .live_session_names
                        .iter()
                        .any(|live_session| live_session == &session_name)
                    {
                        self.live_session_names.push(session_name.clone());
                    }
                    self.add_or_select_worktree_session(
                        &branch,
                        &worktree_path,
                        &session_name,
                        Some(&session_name),
                    );
                    if let Some(registry) = treemin_registry() {
                        let _ = registry.add(&session_name);
                    }
                    self.status = Status::Success(format!(
                        "Created session `{session_name}` for worktree `{}`. Switching...",
                        worktree_path.display()
                    ));
                    switch_session(Some(&session_name));
                } else {
                    self.status = Status::Error(commands::command_error(
                        "Failed to create session.",
                        &stderr,
                    ));
                }
            }
            CommandAction::ListWorktrees => {
                if exit_code == Some(0) {
                    let previous_selection = self.selected_session_key();
                    self.known_worktrees = git::parse_worktree_locations(
                        &String::from_utf8_lossy(&stdout),
                        self.current_worktree_root.as_deref(),
                    );
                    self.rebuild_worktree_sessions_with_selection(previous_selection.as_deref());
                    self.status = Status::Ready;
                } else {
                    self.status = Status::Error(commands::command_error(
                        "Failed to load worktree sessions.",
                        &stderr,
                    ));
                }
            }
            CommandAction::DeleteSession {
                session: session_name,
            } => {
                if exit_code == Some(0) {
                    let worktree_path = self
                        .worktree_sessions
                        .iter()
                        .find(|entry| {
                            entry.live_session_name.as_deref() == Some(session_name.as_str())
                        })
                        .and_then(|entry| {
                            if entry.is_current
                                || self.repo_root.as_deref() == entry.path.as_deref()
                            {
                                None
                            } else {
                                entry.path.clone()
                            }
                        });
                    self.live_session_names
                        .retain(|live_session| live_session != &session_name);
                    if let Some(registry) = treemin_registry() {
                        let _ = registry.remove(&session_name);
                    }
                    if let (Some(repo_root), Some(worktree_path)) =
                        (self.repo_root.clone(), worktree_path)
                    {
                        self.pending_delete = Some(PendingDelete {
                            session_name: Some(session_name.clone()),
                            worktree_path: Some(worktree_path.clone()),
                        });
                        self.status = Status::Busy(format!(
                            "Deleting session `{session_name}` and worktree `{}`...",
                            worktree_path.display()
                        ));
                        git::delete_worktree(repo_root, &worktree_path);
                    } else {
                        self.pending_delete = Some(PendingDelete {
                            session_name: Some(session_name.clone()),
                            worktree_path: None,
                        });
                        let previous_selection = self.selected_session_key();
                        self.rebuild_worktree_sessions_with_selection(
                            previous_selection.as_deref(),
                        );
                        self.status = Status::Ready;
                    }
                } else {
                    self.pending_delete = None;
                    self.status = Status::Error(commands::command_error(
                        "Failed to delete session.",
                        &stderr,
                    ));
                }
            }
            CommandAction::DeleteWorktree { path } => {
                if exit_code == Some(0) {
                    self.known_worktrees
                        .retain(|worktree| worktree.path != path);
                    let previous_selection = self.selected_session_key();
                    self.rebuild_worktree_sessions_with_selection(previous_selection.as_deref());
                    self.status = Status::Ready;
                    self.pending_delete = Some(PendingDelete {
                        session_name: None,
                        worktree_path: Some(path),
                    });
                } else {
                    self.pending_delete = None;
                    self.status = Status::Error(commands::command_error(
                        "Failed to delete worktree.",
                        &stderr,
                    ));
                }
            }
        }
    }

    fn create_worktree(&mut self, branch: &str, branch_exists: bool) {
        let worktree_path = self.worktree_path(branch);

        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        self.status = Status::Busy(format!(
            "Creating worktree `{}`...",
            worktree_path.display()
        ));

        if branch_exists {
            git::create_worktree_existing(repo_root, branch, &worktree_path);
        } else {
            git::create_worktree(
                repo_root,
                branch,
                &worktree_path,
                self.config.base_branch.as_deref(),
            );
        }
    }

    fn discover_repo(&mut self) {
        self.status = Status::Busy("Discovering repository root...".to_string());
        git::discover_repo();
    }

    fn refresh_session_list(&mut self) {
        if let Ok(snapshot) = get_session_list() {
            self.live_session_names = snapshot
                .live_sessions
                .into_iter()
                .map(|session| session.name)
                .collect();
            self.rebuild_worktree_sessions();
        }
    }

    fn load_repo_config(&mut self, repo_root: PathBuf) {
        self.status = Status::Busy("Loading repository configuration...".to_string());
        commands::load_repo_config(repo_root);
    }

    fn refresh_worktree_sessions(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        self.status = Status::Busy("Loading worktree sessions...".to_string());
        git::list_worktrees(repo_root);
    }

    fn select_previous_worktree_session(&mut self) {
        if self.worktree_sessions.is_empty() {
            self.status =
                Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        }

        let Some(selected_index) = self.selected_index else {
            self.status = Status::Ready;
            return;
        };

        if let Some(index) = previous_selectable_index(&self.worktree_sessions, selected_index) {
            self.selected_index = Some(index);
        }
        self.status = Status::Ready;
    }

    fn select_next_worktree_session(&mut self) {
        if self.worktree_sessions.is_empty() {
            self.status =
                Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        }

        let Some(selected_index) = self.selected_index else {
            self.status = Status::Ready;
            return;
        };

        if let Some(index) = next_selectable_index(&self.worktree_sessions, selected_index) {
            self.selected_index = Some(index);
        }
        self.status = Status::Ready;
    }

    fn switch_selected_worktree_session(&mut self) {
        let Some(selected_index) = self.selected_index else {
            self.status = Status::Error(
                "No selectable worktree sessions found for this repository.".to_string(),
            );
            return;
        };

        let Some(entry) = self.worktree_sessions.get(selected_index) else {
            self.status =
                Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        };

        if !entry.has_live_session {
            let Some(worktree_path) = entry.path.clone() else {
                self.status = Status::Error(format!(
                    "No worktree path was found for `{}`.",
                    entry.branch
                ));
                return;
            };
            let branch = entry.branch.clone();
            let session_name = entry.session_name.clone();
            self.create_session_for_worktree(&branch, &worktree_path, &session_name);
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

    fn create_or_switch_worktree_session(&mut self, branch: &str) {
        let worktree_path = self.worktree_path(branch);
        let session_name = self.session_name(branch);
        let sibling_branches = sibling_branches_with(self.known_worktrees.iter(), branch);

        if let Some(live_session_name) = matching_worktree_live_session_name(
            self.repo_name.as_deref(),
            branch,
            &self.config,
            &sibling_branches,
            false,
            &self.live_session_names,
        ) {
            if let Some(registry) = treemin_registry() {
                let _ = registry.add(&live_session_name);
            }
            self.add_or_select_worktree_session(
                branch,
                &worktree_path,
                &session_name,
                Some(&live_session_name),
            );
            self.status = Status::Success(format!(
                "Created worktree `{}`. Switching to session `{}`...",
                worktree_path.display(),
                live_session_name,
            ));
            switch_session(Some(&live_session_name));
            return;
        }

        self.create_session_for_worktree(branch, &worktree_path, &session_name);
    }

    fn create_session_for_worktree(
        &mut self,
        branch: &str,
        worktree_path: &Path,
        session_name: &str,
    ) {
        let worktree_path_string = worktree_path.display().to_string();

        self.status = Status::Busy(format!(
            "Creating session `{session_name}` in `{}`...",
            worktree_path_string
        ));
        zellij::create_session(branch, worktree_path, session_name);
    }

    fn delete_selected_worktree_session(&mut self) {
        let Some(selected_index) = self.selected_index else {
            self.status = Status::Error(
                "No selectable worktree sessions found for this repository.".to_string(),
            );
            return;
        };

        let Some(entry) = self.worktree_sessions.get(selected_index) else {
            self.status =
                Status::Error("No worktree sessions found for this repository.".to_string());
            return;
        };

        if entry.is_current {
            self.status =
                Status::Error("Cannot delete the current session from inside itself.".to_string());
            return;
        }

        let Some(repo_root) = self.repo_root.clone() else {
            self.status = Status::Error("Repository root is not available yet.".to_string());
            return;
        };

        if !entry.has_live_session {
            let Some(worktree_path) = entry.path.as_ref() else {
                self.status = Status::Error(format!(
                    "No worktree path was found for `{}`.",
                    entry.branch
                ));
                return;
            };

            if self.repo_root.as_deref() == Some(worktree_path.as_path()) {
                self.status = Status::Error(
                    "Cannot delete the main repository worktree from treemin.".to_string(),
                );
                return;
            }

            self.pending_delete = Some(PendingDelete {
                session_name: None,
                worktree_path: Some(worktree_path.to_path_buf()),
            });
            self.status = Status::Busy(format!(
                "Deleting worktree `{}`...",
                worktree_path.display()
            ));
            git::delete_worktree(repo_root, worktree_path);
            return;
        }

        let Some(live_session_name) = entry.live_session_name.as_deref() else {
            self.status = Status::Error(format!(
                "No live Zellij session match was found for `{}`.",
                entry.branch
            ));
            return;
        };

        self.pending_delete = Some(PendingDelete {
            session_name: Some(live_session_name.to_string()),
            worktree_path: None,
        });
        self.status = Status::Busy(format!("Deleting session `{live_session_name}`..."));
        zellij::delete_session(repo_root, live_session_name);
    }

    fn add_or_select_worktree_session(
        &mut self,
        branch: &str,
        path: &Path,
        session_name: &str,
        live_session_name: Option<&str>,
    ) {
        let live_session_name = live_session_name.unwrap_or(session_name);

        if let Some(index) = self
            .worktree_sessions
            .iter()
            .position(|entry| entry.live_session_name.as_deref() == Some(live_session_name))
        {
            self.selected_index = if self.worktree_sessions[index].is_current {
                first_selectable_index(&self.worktree_sessions)
            } else {
                Some(index)
            };
            return;
        }

        self.worktree_sessions.push(WorktreeSessionEntry {
            branch: branch.to_string(),
            path: Some(path.to_path_buf()),
            session_name: session_name.to_string(),
            live_session_name: Some(live_session_name.to_string()),
            has_live_session: true,
            is_current: false,
        });
        self.selected_index = Some(self.worktree_sessions.len() - 1);
    }

    fn rebuild_worktree_sessions(&mut self) {
        let previous_selection = self.selected_session_key();
        self.rebuild_worktree_sessions_with_selection(previous_selection.as_deref());
    }

    fn rebuild_worktree_sessions_with_selection(&mut self, previous_selection: Option<&str>) {
        self.worktree_sessions = build_worktree_sessions(
            self.repo_root.as_deref(),
            self.repo_name.as_deref(),
            &self.config,
            &self.known_worktrees,
            &self.live_session_names,
        );
        self.selected_index =
            selected_index_for_sessions(&self.worktree_sessions, previous_selection);
    }

    fn selected_session_key(&self) -> Option<String> {
        self.selected_index
            .and_then(|selected_index| self.worktree_sessions.get(selected_index))
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
        let sibling_branches = sibling_branches_with(self.known_worktrees.iter(), branch);
        naming::session_name(
            self.repo_name.as_deref(),
            branch,
            &self.config,
            &sibling_branches,
            false,
        )
    }
}

fn build_worktree_sessions(
    repo_root: Option<&Path>,
    repo_name: Option<&str>,
    config: &Config,
    known_worktrees: &[WorktreeLocation],
    live_session_names: &[String],
) -> Vec<WorktreeSessionEntry> {
    let Some(repo_name) = repo_name else {
        return Vec::new();
    };
    let sibling_branches = sibling_branches_with(known_worktrees.iter(), "");

    let mut sessions = known_worktrees
        .iter()
        .map(|worktree| {
            let is_main_worktree = repo_root == Some(worktree.path.as_path());
            let generated_live_session_name = matching_worktree_live_session_name(
                Some(repo_name),
                &worktree.branch,
                config,
                &sibling_branches,
                is_main_worktree,
                live_session_names,
            );
            let live_session_name = generated_live_session_name;
            let session_name = naming::session_name(
                Some(repo_name),
                &worktree.branch,
                config,
                &sibling_branches,
                is_main_worktree,
            );

            WorktreeSessionEntry {
                branch: worktree.branch.clone(),
                path: Some(worktree.path.clone()),
                session_name,
                live_session_name: live_session_name.clone(),
                has_live_session: live_session_name.is_some(),
                is_current: worktree.is_current,
            }
        })
        .collect::<Vec<_>>();

    sessions.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| left.branch.cmp(&right.branch))
    });
    sessions
}

fn worktree_session_name_candidates(
    repo_name: Option<&str>,
    branch: &str,
    config: &Config,
    sibling_branches: &[String],
    is_main_worktree: bool,
) -> Vec<String> {
    naming::session_name_candidates(
        repo_name,
        branch,
        config,
        sibling_branches,
        is_main_worktree,
    )
}

fn matching_worktree_live_session_name(
    repo_name: Option<&str>,
    branch: &str,
    config: &Config,
    sibling_branches: &[String],
    is_main_worktree: bool,
    live_session_names: &[String],
) -> Option<String> {
    let candidates = worktree_session_name_candidates(
        repo_name,
        branch,
        config,
        sibling_branches,
        is_main_worktree,
    );
    live_session_names
        .iter()
        .find(|session_name| {
            candidates
                .iter()
                .any(|candidate| candidate == *session_name)
        })
        .cloned()
}

fn sibling_branches_with<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeLocation>,
    extra_branch: &str,
) -> Vec<String> {
    let mut branches = worktrees
        .map(|worktree| worktree.branch.clone())
        .collect::<Vec<_>>();
    if !extra_branch.is_empty() {
        branches.push(extra_branch.to_string());
    }
    branches
}

fn selected_index_for_sessions(
    sessions: &[WorktreeSessionEntry],
    previous_selection: Option<&str>,
) -> Option<usize> {
    if sessions.is_empty() {
        return None;
    }

    if let Some(previous_selection) = previous_selection {
        if let Some(index) = sessions.iter().position(|entry| {
            !entry.is_current && session_selection_key(entry) == previous_selection
        }) {
            return Some(index);
        }
    }

    first_selectable_index(sessions)
}

fn first_selectable_index(sessions: &[WorktreeSessionEntry]) -> Option<usize> {
    sessions.iter().position(|entry| !entry.is_current)
}

fn previous_selectable_index(
    sessions: &[WorktreeSessionEntry],
    current_index: usize,
) -> Option<usize> {
    (0..current_index)
        .rev()
        .find(|index| !sessions[*index].is_current)
        .or_else(|| first_selectable_index(sessions))
}

fn next_selectable_index(sessions: &[WorktreeSessionEntry], current_index: usize) -> Option<usize> {
    sessions
        .iter()
        .enumerate()
        .skip(current_index + 1)
        .find(|(_, entry)| !entry.is_current)
        .map(|(index, _)| index)
        .or_else(|| first_selectable_index(sessions))
}

fn session_selection_key(entry: &WorktreeSessionEntry) -> String {
    entry
        .live_session_name
        .clone()
        .unwrap_or_else(|| entry.session_name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_previous_selection_when_sessions_refresh() {
        let sessions = vec![
            WorktreeSessionEntry {
                branch: "main".to_string(),
                path: Some(PathBuf::from("/tmp/repo")),
                session_name: "repo".to_string(),
                live_session_name: Some("repo".to_string()),
                has_live_session: true,
                is_current: true,
            },
            WorktreeSessionEntry {
                branch: "feature".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo|feature".to_string(),
                live_session_name: Some("repo|feature".to_string()),
                has_live_session: true,
                is_current: false,
            },
        ];

        let selected_index = selected_index_for_sessions(&sessions, Some("repo|feature"));

        assert_eq!(selected_index, Some(1));
    }

    #[test]
    fn skips_current_session_when_picking_default_selection() {
        let sessions = vec![
            WorktreeSessionEntry {
                branch: "main".to_string(),
                path: Some(PathBuf::from("/tmp/repo")),
                session_name: "repo".to_string(),
                live_session_name: Some("repo".to_string()),
                has_live_session: true,
                is_current: true,
            },
            WorktreeSessionEntry {
                branch: "feature".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo|feature".to_string(),
                live_session_name: Some("repo|feature".to_string()),
                has_live_session: true,
                is_current: false,
            },
        ];

        assert_eq!(selected_index_for_sessions(&sessions, None), Some(1));
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
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &worktrees,
            &["repo".to_string(), "repo|feature-test".to_string()],
        );

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].branch, "main");
        assert_eq!(sessions[0].path, Some(PathBuf::from("/tmp/repo")));
        assert_eq!(sessions[0].session_name, "repo");
        assert_eq!(sessions[0].live_session_name.as_deref(), Some("repo"));
        assert_eq!(sessions[1].branch, "feature/test");
        assert_eq!(
            sessions[1].path,
            Some(PathBuf::from("/tmp/repo/.worktrees/feature"))
        );
        assert_eq!(
            sessions[1].live_session_name.as_deref(),
            Some("repo|feature-test")
        );
        assert_eq!(sessions[1].session_name, "repo|feature-test");
    }

    #[test]
    fn current_worktree_session_sorts_first() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[
                WorktreeLocation {
                    branch: "zzz-current".to_string(),
                    path: PathBuf::from("/tmp/repo/.worktrees/zzz-current"),
                    is_current: true,
                },
                WorktreeLocation {
                    branch: "aaa-other".to_string(),
                    path: PathBuf::from("/tmp/repo/.worktrees/aaa-other"),
                    is_current: false,
                },
            ],
            &[],
        );

        assert_eq!(sessions[0].branch, "zzz-current");
        assert!(sessions[0].is_current);
        assert_eq!(sessions[1].branch, "aaa-other");
    }

    #[test]
    fn next_and_previous_selection_skip_current_session() {
        let mut state = State {
            worktree_sessions: vec![
                WorktreeSessionEntry {
                    branch: "main".to_string(),
                    path: Some(PathBuf::from("/tmp/repo")),
                    session_name: "repo".to_string(),
                    live_session_name: Some("repo".to_string()),
                    has_live_session: true,
                    is_current: true,
                },
                WorktreeSessionEntry {
                    branch: "feature-a".to_string(),
                    path: Some(PathBuf::from("/tmp/repo/.worktrees/feature-a")),
                    session_name: "repo|feature-a".to_string(),
                    live_session_name: Some("repo|feature-a".to_string()),
                    has_live_session: true,
                    is_current: false,
                },
                WorktreeSessionEntry {
                    branch: "feature-b".to_string(),
                    path: Some(PathBuf::from("/tmp/repo/.worktrees/feature-b")),
                    session_name: "repo|feature-b".to_string(),
                    live_session_name: Some("repo|feature-b".to_string()),
                    has_live_session: true,
                    is_current: false,
                },
            ],
            selected_index: Some(1),
            ..State::default()
        };

        state.select_previous_worktree_session();
        assert_eq!(state.selected_index, Some(1));

        state.select_next_worktree_session();
        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn keeps_worktree_without_live_session_in_session_list() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[WorktreeLocation {
                branch: "feature/test".to_string(),
                path: PathBuf::from("/tmp/repo/.worktrees/feature"),
                is_current: false,
            }],
            &[],
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].branch, "feature/test");
        assert_eq!(
            sessions[0].path,
            Some(PathBuf::from("/tmp/repo/.worktrees/feature"))
        );
        assert_eq!(sessions[0].session_name, "repo|feature-test");
        assert_eq!(sessions[0].live_session_name, None);
        assert!(!sessions[0].has_live_session);
    }

    #[test]
    fn ignores_live_session_when_worktree_is_missing() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[],
            &["repo|feature-test".to_string()],
        );

        assert!(sessions.is_empty());
    }

    #[test]
    fn matches_plain_repo_name_session_for_main_worktree() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[WorktreeLocation {
                branch: "main".to_string(),
                path: PathBuf::from("/tmp/repo"),
                is_current: true,
            }],
            &["repo".to_string()],
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].branch, "main");
        assert_eq!(sessions[0].session_name, "repo");
        assert_eq!(sessions[0].live_session_name.as_deref(), Some("repo"));
        assert!(sessions[0].has_live_session);
    }

    #[test]
    fn does_not_match_plain_repo_name_session_for_linked_worktree() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[WorktreeLocation {
                branch: "feature/test".to_string(),
                path: PathBuf::from("/tmp/repo/.worktrees/feature"),
                is_current: false,
            }],
            &["repo".to_string()],
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].branch, "feature/test");
        assert_eq!(sessions[0].session_name, "repo|feature-test");
        assert_eq!(sessions[0].live_session_name, None);
        assert!(!sessions[0].has_live_session);
    }

    #[test]
    fn prefers_plain_repo_name_over_generated_session_name_for_main_worktree() {
        let config = Config::default();
        let sessions = build_worktree_sessions(
            Some(Path::new("/tmp/repo")),
            Some("repo"),
            &config,
            &[WorktreeLocation {
                branch: "main".to_string(),
                path: PathBuf::from("/tmp/repo"),
                is_current: true,
            }],
            &["repo".to_string(), "repo-main-17c9aaa7".to_string()],
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_name, "repo");
        assert_eq!(sessions[0].live_session_name.as_deref(), Some("repo"));
    }

    #[test]
    fn create_worktree_session_switches_immediately_when_session_is_live() {
        let mut state = State {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            repo_name: Some("repo".to_string()),
            live_session_names: vec!["repo|feature-test".to_string()],
            ..State::default()
        };

        state.create_or_switch_worktree_session("feature/test");

        assert!(matches!(
            state.status,
            Status::Success(ref message)
                if message == "Created worktree `/tmp/repo/.worktrees/feature/test`. Switching to session `repo|feature-test`..."
        ));
        assert_eq!(state.worktree_sessions.len(), 1);
        assert_eq!(
            state.worktree_sessions[0].live_session_name.as_deref(),
            Some("repo|feature-test")
        );
    }

    #[test]
    fn create_worktree_session_switches_to_matching_legacy_session_for_hyphenated_repo() {
        let config = Config::default();
        let legacy_session_name = naming::session_name_candidates(
            Some("repo-name"),
            "feature/test",
            &config,
            &["feature/test".to_string()],
            false,
        )
        .into_iter()
        .find(|candidate| candidate != "repo-name-feature-test")
        .expect("expected a secondary session name candidate");

        let mut state = State {
            config,
            repo_root: Some(PathBuf::from("/tmp/repo-name")),
            repo_name: Some("repo-name".to_string()),
            live_session_names: vec![legacy_session_name.clone()],
            ..State::default()
        };

        state.create_or_switch_worktree_session("feature/test");

        assert!(matches!(
            state.status,
            Status::Success(ref message)
                if message == &format!(
                    "Created worktree `/tmp/repo-name/.worktrees/feature/test`. Switching to session `{legacy_session_name}`..."
                )
        ));
        assert_eq!(state.worktree_sessions.len(), 1);
        assert_eq!(
            state.worktree_sessions[0].session_name,
            "repo-name|feature-test"
        );
        assert_eq!(
            state.worktree_sessions[0].live_session_name.as_deref(),
            Some(legacy_session_name.as_str())
        );
    }

    #[test]
    fn create_worktree_session_requests_background_creation_when_missing() {
        let mut state = State {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            repo_name: Some("repo".to_string()),
            ..State::default()
        };

        state.create_or_switch_worktree_session("feature/test");

        assert!(matches!(
            state.status,
            Status::Busy(ref message)
                if message == "Creating session `repo|feature-test` in `/tmp/repo/.worktrees/feature/test`..."
        ));
        assert!(state.worktree_sessions.is_empty());
    }

    #[test]
    fn switch_selected_worktree_session_creates_missing_session() {
        let mut state = State {
            worktree_sessions: vec![WorktreeSessionEntry {
                branch: "feature/test".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo|feature-test".to_string(),
                live_session_name: None,
                has_live_session: false,
                is_current: false,
            }],
            selected_index: Some(0),
            ..State::default()
        };

        state.switch_selected_worktree_session();

        assert!(matches!(
            state.status,
            Status::Busy(ref message)
                if message == "Creating session `repo|feature-test` in `/tmp/repo/.worktrees/feature`..."
        ));
    }

    #[test]
    fn delete_action_clears_branch_input_before_deleting_session() {
        let mut state = State {
            branch_input: "feature/test".to_string(),
            status: Status::Success("old status".to_string()),
            ..State::default()
        };

        state.begin_delete_action();

        assert!(state.branch_input.is_empty());
        assert!(matches!(state.status, Status::Ready));
    }

    #[test]
    fn refuses_to_delete_current_session() {
        let mut state = State {
            worktree_sessions: vec![WorktreeSessionEntry {
                branch: "main".to_string(),
                path: Some(PathBuf::from("/tmp/repo")),
                session_name: "repo".to_string(),
                live_session_name: Some("repo".to_string()),
                has_live_session: true,
                is_current: true,
            }],
            ..State::default()
        };

        state.delete_selected_worktree_session();

        assert!(matches!(
            state.status,
            Status::Error(ref message) if message == "Cannot delete the current session from inside itself."
        ));
    }

    #[test]
    fn refuses_to_delete_session_without_live_match() {
        let mut state = State {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            worktree_sessions: vec![WorktreeSessionEntry {
                branch: "feature/test".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo|feature-test".to_string(),
                live_session_name: None,
                has_live_session: false,
                is_current: false,
            }],
            ..State::default()
        };

        state.delete_selected_worktree_session();

        assert!(matches!(
            state.status,
            Status::Busy(ref message) if message == "Deleting worktree `/tmp/repo/.worktrees/feature`..."
        ));
    }

    #[test]
    fn refuses_to_delete_main_worktree_without_live_session() {
        let mut state = State {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            worktree_sessions: vec![WorktreeSessionEntry {
                branch: "main".to_string(),
                path: Some(PathBuf::from("/tmp/repo")),
                session_name: "repo".to_string(),
                live_session_name: None,
                has_live_session: false,
                is_current: false,
            }],
            ..State::default()
        };

        state.delete_selected_worktree_session();

        assert!(matches!(
            state.status,
            Status::Error(ref message)
                if message == "Cannot delete the main repository worktree from treemin."
        ));
    }

    #[test]
    fn deleting_linked_session_also_deletes_worktree() {
        let mut state = State {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            worktree_sessions: vec![WorktreeSessionEntry {
                branch: "feature/test".to_string(),
                path: Some(PathBuf::from("/tmp/repo/.worktrees/feature")),
                session_name: "repo|feature-test".to_string(),
                live_session_name: Some("repo|feature-test".to_string()),
                has_live_session: true,
                is_current: false,
            }],
            live_session_names: vec!["repo|feature-test".to_string()],
            ..State::default()
        };

        state.handle_run_command_result(
            Some(0),
            Vec::new(),
            Vec::new(),
            BTreeMap::from([
                ("action".to_string(), "delete-session".to_string()),
                ("session".to_string(), "repo|feature-test".to_string()),
            ]),
        );

        assert!(matches!(
            state.status,
            Status::Busy(ref message)
                if message == "Deleting session `repo|feature-test` and worktree `/tmp/repo/.worktrees/feature`..."
        ));
        assert!(state.live_session_names.is_empty());
    }
}
