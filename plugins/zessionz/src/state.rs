use std::collections::BTreeMap;

use humantime::format_duration;
use zellij_tile::prelude::*;

use crate::config::Config;
use crate::session::{SessionItem, SessionManager};
use crate::ui;
use crate::zoxide::{self, SearchEngine, ZoxideDirectory};

const ZOXIDE_QUERY_CONTEXT: &str = "zoxide_query";
const MAX_SESSION_NAME_LEN: usize = 108;

pub struct State {
    pub(crate) config: Config,
    pub(crate) status: Status,
    pub(crate) active_screen: ActiveScreen,
    pub(crate) draft_session: Option<DraftSession>,
    session_manager: SessionManager,
    directories: Vec<ZoxideDirectory>,
    search_engine: SearchEngine,
    selected_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Loading,
    Busy(String),
    Error(String),
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveScreen {
    Main,
    NewSession,
}

#[derive(Clone)]
pub struct DraftSession {
    pub directory: String,
    pub session_name: String,
    pub layouts: Vec<LayoutInfo>,
    pub selected_layout_index: usize,
}

impl DraftSession {
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            config: Config::default(),
            status: Status::Loading,
            active_screen: ActiveScreen::Main,
            draft_session: None,
            session_manager: SessionManager::default(),
            directories: Vec::new(),
            search_engine: SearchEngine::default(),
            selected_index: 0,
        }
    }
}

impl State {
    pub fn load_plugin(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from_kdl(configuration);
        self.status = Status::Loading;
        self.active_screen = ActiveScreen::Main;
        self.draft_session = None;
        set_selectable(true);
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
            EventType::SessionUpdate,
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
            Event::SessionUpdate(sessions, resurrectable_sessions) => {
                self.session_manager.update_sessions(sessions);
                self.session_manager
                    .update_resurrectable_sessions(resurrectable_sessions);
                self.refresh_search();
                self.clamp_selection();
                true
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    pub fn render_plugin(&mut self, rows: usize, cols: usize) {
        ui::render(self, rows, cols);
    }

    pub(crate) fn display_items(&self) -> Vec<SessionItem> {
        if self.search_engine.is_searching() {
            return self
                .search_engine
                .results()
                .iter()
                .map(|result| result.item.clone())
                .collect();
        }

        let mut items = Vec::new();

        for session in self.session_manager.sessions() {
            if let Some(directory) = matching_directory(
                &session.name,
                &self.directories,
                &self.config.session_separator,
            ) {
                items.push(SessionItem::ExistingSession {
                    name: session.name.clone(),
                    directory: directory.directory.clone(),
                    is_current: session.is_current_session,
                });
            }
        }

        if self.config.show_resurrectable_sessions {
            for (name, duration) in self.session_manager.resurrectable_sessions() {
                if matching_directory(name, &self.directories, &self.config.session_separator)
                    .is_some()
                {
                    items.push(SessionItem::ResurrectableSession {
                        name: name.clone(),
                        duration_text: format!("created {} ago", format_duration(*duration)),
                    });
                }
            }
        }

        for directory in &self.directories {
            items.push(SessionItem::Directory {
                path: directory.directory.clone(),
                session_name: directory.session_name.clone(),
            });
        }

        items
    }

    pub(crate) fn selected_index(&self) -> usize {
        if self.search_engine.is_searching() {
            self.search_engine.selected_index().unwrap_or(0)
        } else {
            self.selected_index
        }
    }

    pub(crate) fn search_term(&self) -> &str {
        self.search_engine.search_term()
    }

    pub(crate) fn directory_count(&self) -> usize {
        self.directories.len()
    }

    pub(crate) fn session_count(&self) -> usize {
        self.session_manager.sessions().len()
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
        if !context.contains_key(ZOXIDE_QUERY_CONTEXT) {
            return;
        }

        if exit_code == Some(0) {
            let output = String::from_utf8_lossy(&stdout);
            self.directories = zoxide::parse_directories(&output, &self.config);
            self.refresh_search();
            self.clamp_selection();
            self.status = Status::Ready;
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

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        match self.active_screen {
            ActiveScreen::Main => self.handle_main_key(key),
            ActiveScreen::NewSession => self.handle_new_session_key(key),
        }
    }

    fn handle_main_key(&mut self, key: KeyWithModifier) -> bool {
        match key.bare_key {
            BareKey::Up if key.has_no_modifiers() => {
                self.move_selection_up();
                true
            }
            BareKey::Down if key.has_no_modifiers() => {
                self.move_selection_down();
                true
            }
            BareKey::Enter if key.has_no_modifiers() => {
                self.handle_enter_on_main();
                true
            }
            BareKey::Enter if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.quick_create_selected_item();
                true
            }
            BareKey::Backspace if key.has_no_modifiers() => {
                let items = self.unfiltered_items();
                self.search_engine.backspace(&items);
                true
            }
            BareKey::Esc if key.has_no_modifiers() => {
                if self.search_engine.is_searching() {
                    self.search_engine.clear();
                    self.clamp_selection();
                    true
                } else {
                    hide_self();
                    false
                }
            }
            BareKey::Char('c') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                hide_self();
                false
            }
            BareKey::Char('r') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.status = Status::Busy("Refreshing zoxide directories...".to_string());
                self.fetch_zoxide_directories();
                true
            }
            BareKey::Char(character) if key.has_no_modifiers() && !character.is_control() => {
                let items = self.unfiltered_items();
                self.search_engine.add_char(character, &items);
                true
            }
            _ => false,
        }
    }

    fn handle_new_session_key(&mut self, key: KeyWithModifier) -> bool {
        let Some(draft) = self.draft_session.as_mut() else {
            self.active_screen = ActiveScreen::Main;
            return true;
        };

        match key.bare_key {
            BareKey::Up if key.has_no_modifiers() => {
                if draft.selected_layout_index == 0 {
                    draft.selected_layout_index = draft.layouts.len();
                } else {
                    draft.selected_layout_index -= 1;
                }
                true
            }
            BareKey::Down if key.has_no_modifiers() => {
                if draft.selected_layout_index >= draft.layouts.len() {
                    draft.selected_layout_index = 0;
                } else {
                    draft.selected_layout_index += 1;
                }
                true
            }
            BareKey::Backspace if key.has_no_modifiers() => {
                draft.session_name.pop();
                true
            }
            BareKey::Esc if key.has_no_modifiers() => {
                self.active_screen = ActiveScreen::Main;
                self.draft_session = None;
                true
            }
            BareKey::Enter if key.has_no_modifiers() => {
                self.create_draft_session(false);
                true
            }
            BareKey::Enter if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.create_draft_session(true);
                true
            }
            BareKey::Char(character) if key.has_no_modifiers() && !character.is_control() => {
                draft.session_name.push(character);
                true
            }
            _ => false,
        }
    }

    fn handle_enter_on_main(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };

        match item {
            SessionItem::ExistingSession { name, .. }
            | SessionItem::ResurrectableSession { name, .. } => {
                switch_session(Some(&name));
                hide_self();
            }
            SessionItem::Directory { path, session_name } => {
                let next_name = self
                    .session_manager
                    .generate_incremented_name(&session_name, &self.config.session_separator);
                self.draft_session = Some(DraftSession {
                    directory: path,
                    session_name: next_name,
                    layouts: self.session_manager.current_layouts(),
                    selected_layout_index: 0,
                });
                self.active_screen = ActiveScreen::NewSession;
                self.status = Status::Ready;
            }
        }
    }

    fn quick_create_selected_item(&mut self) {
        let Some(item) = self.selected_item() else {
            self.status = Status::Error("Select a directory or session first.".to_string());
            return;
        };

        match item {
            SessionItem::ExistingSession { name, .. }
            | SessionItem::ResurrectableSession { name, .. } => {
                switch_session(Some(&name));
                hide_self();
            }
            SessionItem::Directory { path, session_name } => {
                let next_name = self
                    .session_manager
                    .generate_incremented_name(&session_name, &self.config.session_separator);
                if let Err(message) = validate_session_name(&next_name) {
                    self.status = Status::Error(message);
                    return;
                }

                let cwd = Some(std::path::PathBuf::from(path));
                if let Some(layout_name) = self.config.default_layout.as_deref() {
                    if let Some(layout) = self
                        .session_manager
                        .current_layouts()
                        .into_iter()
                        .find(|layout| layout.name() == layout_name)
                    {
                        switch_session_with_layout(Some(&next_name), layout, cwd);
                    } else {
                        switch_session_with_cwd(Some(&next_name), cwd);
                    }
                } else {
                    switch_session_with_cwd(Some(&next_name), cwd);
                }
                hide_self();
            }
        }
    }

    fn create_draft_session(&mut self, use_default_layout: bool) {
        let Some(draft) = self.draft_session.clone() else {
            return;
        };

        if let Err(message) = validate_session_name(&draft.session_name) {
            self.status = Status::Error(message);
            return;
        }

        if self
            .session_manager
            .current_session_name()
            .is_some_and(|name| name == draft.session_name)
        {
            self.status = Status::Error(
                "Cannot create a session with the current session name.".to_string(),
            );
            return;
        }

        let cwd = Some(std::path::PathBuf::from(&draft.directory));
        if use_default_layout {
            if let Some(layout_name) = self.config.default_layout.as_deref() {
                if let Some(layout) = draft
                    .layouts
                    .iter()
                    .find(|layout| layout.name() == layout_name)
                    .cloned()
                {
                    switch_session_with_layout(Some(&draft.session_name), layout, cwd);
                } else {
                    switch_session_with_cwd(Some(&draft.session_name), cwd);
                }
            } else {
                switch_session_with_cwd(Some(&draft.session_name), cwd);
            }
        } else if draft.selected_layout_index == 0 {
            switch_session_with_cwd(Some(&draft.session_name), cwd);
        } else if let Some(layout) = draft.layouts.get(draft.selected_layout_index - 1).cloned() {
            switch_session_with_layout(Some(&draft.session_name), layout, cwd);
        } else {
            switch_session_with_cwd(Some(&draft.session_name), cwd);
        }

        hide_self();
    }

    fn fetch_zoxide_directories(&self) {
        run_command(
            &["zoxide", "query", "-l", "-s"],
            BTreeMap::from([(ZOXIDE_QUERY_CONTEXT.to_string(), "true".to_string())]),
        );
    }

    fn refresh_search(&mut self) {
        if self.search_engine.is_searching() {
            let items = self.unfiltered_items();
            self.search_engine.refresh(&items);
        }
    }

    fn unfiltered_items(&self) -> Vec<SessionItem> {
        let mut clone = Self {
            search_engine: SearchEngine::default(),
            selected_index: self.selected_index,
            draft_session: self.draft_session.clone(),
            status: self.status.clone(),
            active_screen: self.active_screen,
            config: self.config.clone(),
            session_manager: SessionManager::default(),
            directories: self.directories.clone(),
        };
        clone.session_manager.update_sessions(self.session_manager.sessions().to_vec());
        clone.session_manager.update_resurrectable_sessions(
            self.session_manager.resurrectable_sessions().to_vec(),
        );
        clone.display_items()
    }

    fn selected_item(&self) -> Option<SessionItem> {
        if self.search_engine.is_searching() {
            self.search_engine.selected_item().cloned()
        } else {
            self.display_items().get(self.selected_index).cloned()
        }
    }

    fn move_selection_up(&mut self) {
        if self.search_engine.is_searching() {
            self.search_engine.move_up();
            return;
        }

        let items_len = self.display_items().len();
        if items_len == 0 {
            return;
        }

        if self.selected_index == 0 {
            self.selected_index = items_len - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.search_engine.is_searching() {
            self.search_engine.move_down();
            return;
        }

        let items_len = self.display_items().len();
        if items_len == 0 {
            return;
        }

        self.selected_index = (self.selected_index + 1) % items_len;
    }

    fn clamp_selection(&mut self) {
        let items_len = self.display_items().len();
        if items_len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= items_len {
            self.selected_index = items_len - 1;
        }
    }
}

fn matching_directory<'a>(
    session_name: &str,
    directories: &'a [ZoxideDirectory],
    separator: &str,
) -> Option<&'a ZoxideDirectory> {
    directories.iter().find(|directory| {
        directory.session_name == session_name
            || session_name
                .strip_prefix(&directory.session_name)
                .and_then(|suffix| suffix.strip_prefix(separator))
                .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
    })
}

fn validate_session_name(session_name: &str) -> Result<(), String> {
    if session_name.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    if session_name.contains('/') {
        return Err("Session name cannot contain '/'.".to_string());
    }
    if session_name.len() >= MAX_SESSION_NAME_LEN {
        return Err(format!(
            "Session name must be shorter than {MAX_SESSION_NAME_LEN} bytes."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_incremented_session_names() {
        let directories = vec![ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];

        assert!(matching_directory("repo.3", &directories, ".").is_some());
        assert!(matching_directory("repo-x", &directories, ".").is_none());
    }

    #[test]
    fn validates_session_names() {
        assert!(validate_session_name("dev").is_ok());
        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("dev/test").is_err());
    }
}
