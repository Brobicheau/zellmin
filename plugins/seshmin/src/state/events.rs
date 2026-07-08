use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use super::{ActiveScreen, State, Status};
use crate::config::Config;
use crate::storage::treemin_registry;
use crate::ui;
use crate::zoxide;

const ZOXIDE_QUERY_CONTEXT: &str = "zoxide_query";

impl State {
    pub fn load_plugin(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from_kdl(configuration);
        self.status = Status::Loading;
        self.active_screen = ActiveScreen::Main;
        self.draft_session = None;
        self.sessions_loaded = false;
        self.directories_loaded = false;
        rename_plugin_pane(get_plugin_ids().plugin_id, "seshmin");
        set_selectable(true);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
            EventType::Key,
        ]);
        request_permission(&[
            PermissionType::RunCommands,
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
    }

    pub fn update_plugin(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(permission_status) => {
                self.handle_permission_result(permission_status);
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

    pub fn render_plugin(&mut self, rows: usize, cols: usize) {
        ui::render(self, rows, cols);
    }

    fn handle_permission_result(&mut self, permission_status: PermissionStatus) {
        match permission_status {
            PermissionStatus::Granted => {
                self.status = Status::Busy("Loading zoxide directories...".to_string());
                self.fetch_zoxide_directories();
            }
            PermissionStatus::Denied => {
                self.status = Status::Error(
                    "Permission request was denied. Reload the plugin and grant access."
                        .to_string(),
                );
            }
        }
    }

    fn handle_run_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        context: BTreeMap<String, String>,
    ) {
        if context.contains_key(ZOXIDE_QUERY_CONTEXT) {
            self.handle_zoxide_result(exit_code, stdout, stderr);
        }
    }

    fn handle_zoxide_result(&mut self, exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) {
        if exit_code == Some(0) {
            let output = String::from_utf8_lossy(&stdout);
            self.directories = zoxide::parse_directories(&output, &self.config);
            self.directories_loaded = true;
            self.refresh_search();
            self.clamp_selection();
            if self.refresh_session_list() {
                self.sync_status();
            }
        } else {
            let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                "Is zoxide installed and on PATH?".to_string()
            } else {
                stderr
            };
            self.status = Status::Error(format!("Failed to run zoxide. {detail}"));
        }
    }

    fn refresh_session_list(&mut self) -> bool {
        match get_session_list() {
            Ok(snapshot) => {
                self.session_manager.update_sessions(snapshot.live_sessions);
                self.session_manager
                    .update_resurrectable_sessions(snapshot.resurrectable_sessions);
                self.filter_treemin_sessions();
                self.sessions_loaded = true;
                self.selected_index = 0;
                self.refresh_search();
                self.clamp_selection();
                true
            }
            Err(error) => {
                self.status = Status::Error(format!("Failed to list zellij sessions. {error}"));
                false
            }
        }
    }

    pub(super) fn fetch_zoxide_directories(&self) {
        run_command(
            &["zoxide", "query", "-l", "-s"],
            BTreeMap::from([(ZOXIDE_QUERY_CONTEXT.to_string(), "true".to_string())]),
        );
    }

    pub(super) fn sync_status(&mut self) {
        self.status = if self.sessions_loaded && self.directories_loaded {
            Status::Ready
        } else if self.directories_loaded {
            Status::Busy("Loading sessions...".to_string())
        } else if self.sessions_loaded {
            Status::Busy("Loading zoxide directories...".to_string())
        } else {
            Status::Loading
        };
    }

    fn filter_treemin_sessions(&mut self) {
        let Some(registry) = treemin_registry() else {
            return;
        };
        let Ok(managed_sessions) = registry.list() else {
            return;
        };

        self.filter_managed_sessions(&managed_sessions);
    }

    pub(super) fn filter_managed_sessions(
        &mut self,
        managed_sessions: &std::collections::BTreeSet<String>,
    ) {
        if managed_sessions.is_empty() {
            return;
        }

        self.session_manager
            .retain_sessions(|session| !managed_sessions.contains(&session.name));
        self.session_manager
            .retain_resurrectable_sessions(|(name, _)| !managed_sessions.contains(name));
    }
}
