use std::collections::BTreeMap;

use humantime::format_duration;
use zellij_tile::prelude::*;

use crate::config::Config;
use crate::session::{SessionItem, SessionManager};
use crate::storage::treemin_registry;
use crate::ui;
use crate::zoxide::{self, SearchEngine, ZoxideDirectory};

const ZOXIDE_QUERY_CONTEXT: &str = "zoxide_query";
pub struct State {
    pub(crate) config: Config,
    pub(crate) status: Status,
    pub(crate) active_screen: ActiveScreen,
    pub(crate) draft_session: Option<DraftSession>,
    session_manager: SessionManager,
    directories: Vec<ZoxideDirectory>,
    search_engine: SearchEngine,
    selected_index: usize,
    pub(crate) show_help: bool,
    pub(crate) item_filter: ItemFilter,
    sessions_loaded: bool,
    directories_loaded: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFilter {
    All,
    ZoxideOnly,
    NonZoxideOnly,
}

impl ItemFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::ZoxideOnly,
            Self::ZoxideOnly => Self::NonZoxideOnly,
            Self::NonZoxideOnly => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ZoxideOnly => "zoxide only",
            Self::NonZoxideOnly => "non-zoxide only",
        }
    }
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
            show_help: false,
            item_filter: ItemFilter::All,
            sessions_loaded: false,
            directories_loaded: false,
        }
    }
}

impl State {
    pub fn load_plugin(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from_kdl(configuration);
        self.status = Status::Loading;
        self.active_screen = ActiveScreen::Main;
        self.draft_session = None;
        self.item_filter = ItemFilter::All;
        self.sessions_loaded = false;
        self.directories_loaded = false;
        rename_plugin_pane(get_plugin_ids().plugin_id, "seshmin");
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
                self.filter_treemin_sessions();
                self.sessions_loaded = true;
                self.refresh_search();
                self.clamp_selection();
                self.sync_status();
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
            let directory = matching_directory(
                &session.name,
                &self.directories,
                &self.config.session_separator,
            );
            items.push(SessionItem::ExistingSession {
                name: session.name.clone(),
                directory: directory
                    .map(|directory| directory.directory.clone())
                    .unwrap_or_default(),
                is_current: session.is_current_session,
                is_directory_session: directory.is_some(),
                zoxide_ranking: directory.map(|directory| directory.ranking),
            });
        }

        if self.config.show_resurrectable_sessions {
            for (name, duration) in self.session_manager.resurrectable_sessions() {
                let directory =
                    matching_directory(name, &self.directories, &self.config.session_separator);
                items.push(SessionItem::ResurrectableSession {
                    name: name.clone(),
                    duration_text: format!("created {} ago", format_duration(*duration)),
                    is_directory_session: directory.is_some(),
                    zoxide_ranking: directory.map(|directory| directory.ranking),
                });
            }
        }

        for directory in &self.directories {
            items.push(SessionItem::Directory {
                path: directory.directory.clone(),
                session_name: directory.session_name.clone(),
                zoxide_ranking: directory.ranking,
            });
        }

        items.retain(|item| match self.item_filter {
            ItemFilter::All => true,
            ItemFilter::ZoxideOnly => item.is_zoxide_item(),
            ItemFilter::NonZoxideOnly => !item.is_zoxide_item(),
        });

        sort_items(&mut items);

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
            self.directories_loaded = true;
            self.refresh_search();
            self.clamp_selection();
            self.sync_status();
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
        if matches!(self.status, Status::Error(_))
            && matches!(key.bare_key, BareKey::Esc)
            && key.has_no_modifiers()
        {
            self.sync_status();
            return true;
        }

        if !matches!(self.status, Status::Ready | Status::Error(_)) {
            return false;
        }

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
            BareKey::Char('d') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.delete_selected_item();
                true
            }
            BareKey::Char('h') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.show_help = !self.show_help;
                true
            }
            BareKey::Char('f') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.item_filter = self.item_filter.next();
                self.refresh_search();
                self.clamp_selection();
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
                self.directories_loaded = false;
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
            SessionItem::Directory {
                path, session_name, ..
            } => {
                if self.config.default_layout.is_some() {
                    self.create_directory_session(path, session_name, true);
                } else {
                    let next_name = self.session_manager.generate_incremented_name(
                        &session_name,
                        &self.config.session_separator,
                        zoxide::MAX_SESSION_NAME_LEN,
                    );
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
            SessionItem::Directory {
                path, session_name, ..
            } => {
                self.create_directory_session(path, session_name, true);
            }
        }
    }

    fn create_directory_session(
        &mut self,
        path: String,
        session_name: String,
        use_default_layout: bool,
    ) {
        if let Some(resurrectable_session_name) = self
            .session_manager
            .resurrectable_session_name(&session_name)
        {
            switch_session(Some(resurrectable_session_name));
            hide_self();
            return;
        }

        let next_name = self.session_manager.generate_incremented_name(
            &session_name,
            &self.config.session_separator,
            zoxide::MAX_SESSION_NAME_LEN,
        );
        if let Err(message) = validate_session_name(&next_name) {
            self.status = Status::Error(message);
            return;
        }

        let cwd = Some(std::path::PathBuf::from(path));
        if use_default_layout {
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
        } else {
            switch_session_with_cwd(Some(&next_name), cwd);
        }
        hide_self();
    }

    fn delete_selected_item(&mut self) {
        let Some(item) = self.selected_item() else {
            self.status = Status::Error("Select a session first.".to_string());
            return;
        };

        match item {
            SessionItem::Directory { .. } => {
                self.status = Status::Error(
                    "Select a live or resurrectable session to delete it.".to_string(),
                );
            }
            SessionItem::ExistingSession {
                is_current: true, ..
            } => {
                self.status = Status::Error(
                    "Cannot delete the current session from inside itself.".to_string(),
                );
            }
            SessionItem::ExistingSession { name, .. }
            | SessionItem::ResurrectableSession { name, .. } => {
                self.session_manager.delete_session(&name);
                self.status = Status::Ready;
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
            self.status =
                Status::Error("Cannot create a session with the current session name.".to_string());
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

    fn sync_status(&mut self) {
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

    fn filter_managed_sessions(&mut self, managed_sessions: &std::collections::BTreeSet<String>) {
        if managed_sessions.is_empty() {
            return;
        }

        self.session_manager
            .retain_sessions(|session| !managed_sessions.contains(&session.name));
        self.session_manager
            .retain_resurrectable_sessions(|(name, _)| !managed_sessions.contains(name));
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
            show_help: self.show_help,
            item_filter: self.item_filter,
            sessions_loaded: self.sessions_loaded,
            directories_loaded: self.directories_loaded,
            draft_session: self.draft_session.clone(),
            status: self.status.clone(),
            active_screen: self.active_screen,
            config: self.config.clone(),
            session_manager: SessionManager::default(),
            directories: self.directories.clone(),
        };
        clone
            .session_manager
            .update_sessions(self.session_manager.sessions().to_vec());
        clone
            .session_manager
            .update_resurrectable_sessions(self.session_manager.resurrectable_sessions().to_vec());
        clone.display_items()
    }

    fn selected_item(&self) -> Option<SessionItem> {
        if self.search_engine.is_searching() {
            self.search_engine.selected_item().cloned()
        } else {
            self.display_items()
                .get(self.selected_index)
                .filter(|item| item.is_selectable())
                .cloned()
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

        let items = self.display_items();
        for step in 1..=items_len {
            let index = (self.selected_index + items_len - step) % items_len;
            if items[index].is_selectable() {
                self.selected_index = index;
                break;
            }
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

        let items = self.display_items();
        for step in 1..=items_len {
            let index = (self.selected_index + step) % items_len;
            if items[index].is_selectable() {
                self.selected_index = index;
                break;
            }
        }
    }

    fn clamp_selection(&mut self) {
        let items = self.display_items();
        let items_len = items.len();
        if items_len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= items_len {
            self.selected_index = items_len - 1;
        }

        if items_len > 0 && !items[self.selected_index].is_selectable() {
            if let Some(index) = items.iter().position(SessionItem::is_selectable) {
                self.selected_index = index;
            }
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
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
                })
    })
}

fn sort_items(items: &mut [SessionItem]) {
    items.sort_by(|left, right| left.compare_for_display(right));
}

fn validate_session_name(session_name: &str) -> Result<(), String> {
    if session_name.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    if session_name.contains('/') {
        return Err("Session name cannot contain '/'.".to_string());
    }
    if session_name.len() > zoxide::MAX_SESSION_NAME_LEN {
        return Err(format!(
            "Session name must be at most {} bytes.",
            zoxide::MAX_SESSION_NAME_LEN
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::storage::test_treemin_registry;

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
        assert!(validate_session_name(&"a".repeat(zoxide::MAX_SESSION_NAME_LEN)).is_ok());
        assert!(validate_session_name(&"a".repeat(zoxide::MAX_SESSION_NAME_LEN + 1)).is_err());
    }

    #[test]
    fn enter_opens_layout_picker_only_without_default_layout() {
        let mut state = State::default();
        state.directories = vec![ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];

        state.handle_enter_on_main();

        assert!(state.draft_session.is_some());
        assert_eq!(state.active_screen, ActiveScreen::NewSession);
    }

    #[test]
    fn enter_skips_layout_picker_when_default_layout_is_set() {
        let mut state = State::default();
        state.config.default_layout = Some("dev".to_string());
        state.directories = vec![ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];

        state.handle_enter_on_main();

        assert!(state.draft_session.is_none());
        assert_eq!(state.active_screen, ActiveScreen::Main);
    }

    #[test]
    fn delete_requires_selected_session_not_directory() {
        let mut state = State::default();
        state.directories = vec![ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];

        state.delete_selected_item();

        assert!(matches!(
            state.status,
            Status::Error(ref message)
                if message == "Select a live or resurrectable session to delete it."
        ));
    }

    #[test]
    fn delete_refuses_current_session() {
        let mut state = State::default();
        state.directories = vec![ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];
        state.session_manager.update_sessions(vec![SessionInfo {
            name: "repo".to_string(),
            is_current_session: true,
            ..SessionInfo::default()
        }]);

        state.delete_selected_item();

        assert!(matches!(
            state.status,
            Status::Error(ref message)
                if message == "Cannot delete the current session from inside itself."
        ));
    }

    #[test]
    fn current_session_is_shown_but_not_selected() {
        let mut state = State::default();
        state.directories = vec![ZoxideDirectory {
            ranking: 2.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];
        state.session_manager.update_sessions(vec![
            SessionInfo {
                name: "repo".to_string(),
                is_current_session: true,
                ..SessionInfo::default()
            },
            SessionInfo {
                name: "other-live".to_string(),
                ..SessionInfo::default()
            },
        ]);

        let items = state.display_items();

        assert!(items.iter().any(|item| matches!(
            item,
            SessionItem::ExistingSession {
                name,
                is_current: true,
                ..
            } if name == "repo"
        )));

        state.clamp_selection();

        assert!(matches!(
            state.selected_item(),
            Some(SessionItem::ExistingSession {
                name,
                is_current: false,
                ..
            }) if name == "other-live"
        ));

        assert!(matches!(
            state.display_items().first(),
            Some(SessionItem::ExistingSession {
                name,
                is_current: true,
                ..
            }) if name == "repo"
        ));
    }

    #[test]
    fn current_session_only_list_has_no_selectable_item() {
        let mut state = State::default();
        state.session_manager.update_sessions(vec![SessionInfo {
            name: "repo".to_string(),
            is_current_session: true,
            ..SessionInfo::default()
        }]);

        state.clamp_selection();

        assert!(state.selected_item().is_none());
        assert!(state.display_items().iter().any(|item| matches!(
            item,
            SessionItem::ExistingSession {
                name,
                is_current: true,
                ..
            } if name == "repo"
        )));
    }

    #[test]
    fn shows_sessions_even_without_matching_directory() {
        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.session_manager.update_sessions(vec![SessionInfo {
            name: "loose-live".to_string(),
            is_current_session: false,
            ..SessionInfo::default()
        }]);
        state.session_manager.update_resurrectable_sessions(vec![(
            "loose-dead".to_string(),
            std::time::Duration::from_secs(1),
        )]);

        let items = state.display_items();

        assert!(items.iter().any(|item| matches!(
            item,
            SessionItem::ExistingSession { name, .. } if name == "loose-live"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            SessionItem::ResurrectableSession { name, .. } if name == "loose-dead"
        )));
    }

    #[test]
    fn active_sessions_sort_before_other_items() {
        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.directories = vec![
            ZoxideDirectory {
                ranking: 2.0,
                directory: "/tmp/repo".to_string(),
                session_name: "repo".to_string(),
            },
            ZoxideDirectory {
                ranking: 1.0,
                directory: "/tmp/other".to_string(),
                session_name: "other".to_string(),
            },
        ];
        state.session_manager.update_sessions(vec![
            SessionInfo {
                name: "loose-live".to_string(),
                ..SessionInfo::default()
            },
            SessionInfo {
                name: "repo".to_string(),
                ..SessionInfo::default()
            },
        ]);
        state.session_manager.update_resurrectable_sessions(vec![
            ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
            ("other".to_string(), std::time::Duration::from_secs(1)),
        ]);

        let items = state.display_items();
        let loose_live_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::ExistingSession { name, .. } if name == "loose-live"))
            .unwrap();
        let loose_dead_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "loose-dead"))
            .unwrap();
        let repo_index = items
            .iter()
            .position(
                |item| matches!(item, SessionItem::ExistingSession { name, .. } if name == "repo"),
            )
            .unwrap();
        let other_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "other"))
            .unwrap();

        assert!(repo_index < loose_live_index);
        assert!(loose_live_index < other_index);
        assert!(other_index < loose_dead_index);
    }

    #[test]
    fn non_active_items_sort_by_zoxide_ranking() {
        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.directories = vec![
            ZoxideDirectory {
                ranking: 9.0,
                directory: "/tmp/high".to_string(),
                session_name: "high".to_string(),
            },
            ZoxideDirectory {
                ranking: 3.0,
                directory: "/tmp/mid".to_string(),
                session_name: "mid".to_string(),
            },
            ZoxideDirectory {
                ranking: 1.0,
                directory: "/tmp/low".to_string(),
                session_name: "low".to_string(),
            },
        ];
        state.session_manager.update_resurrectable_sessions(vec![
            ("mid".to_string(), std::time::Duration::from_secs(1)),
            ("loose".to_string(), std::time::Duration::from_secs(1)),
        ]);

        let items = state.display_items();
        let high_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::Directory { session_name, .. } if session_name == "high"))
            .unwrap();
        let mid_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "mid"))
            .unwrap();
        let low_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::Directory { session_name, .. } if session_name == "low"))
            .unwrap();
        let loose_index = items
            .iter()
            .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "loose"))
            .unwrap();

        assert!(high_index < mid_index);
        assert!(mid_index < low_index);
        assert!(low_index < loose_index);
    }

    #[test]
    fn zoxide_only_filter_keeps_only_directory_backed_items() {
        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.directories = vec![ZoxideDirectory {
            ranking: 2.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];
        state.session_manager.update_sessions(vec![
            SessionInfo {
                name: "repo".to_string(),
                ..SessionInfo::default()
            },
            SessionInfo {
                name: "loose-live".to_string(),
                ..SessionInfo::default()
            },
        ]);
        state.session_manager.update_resurrectable_sessions(vec![
            ("repo.2".to_string(), std::time::Duration::from_secs(1)),
            ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
        ]);
        state.item_filter = ItemFilter::ZoxideOnly;

        let items = state.display_items();

        assert!(!items.is_empty());
        assert!(items.iter().all(SessionItem::is_zoxide_item));
    }

    #[test]
    fn non_zoxide_filter_hides_directory_backed_items() {
        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.directories = vec![ZoxideDirectory {
            ranking: 2.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        }];
        state.session_manager.update_sessions(vec![
            SessionInfo {
                name: "repo".to_string(),
                ..SessionInfo::default()
            },
            SessionInfo {
                name: "loose-live".to_string(),
                ..SessionInfo::default()
            },
        ]);
        state.session_manager.update_resurrectable_sessions(vec![
            ("repo.2".to_string(), std::time::Duration::from_secs(1)),
            ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
        ]);
        state.item_filter = ItemFilter::NonZoxideOnly;

        let items = state.display_items();

        assert!(!items.is_empty());
        assert!(items.iter().all(|item| !item.is_zoxide_item()));
    }

    #[test]
    fn filters_out_treemin_managed_sessions() {
        let root = std::env::temp_dir().join(format!(
            "seshmin-registry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let registry = test_treemin_registry(&root);
        registry.add("repo-feature-a").unwrap();

        let mut state = State::default();
        state.config.show_resurrectable_sessions = true;
        state.session_manager.update_sessions(vec![
            SessionInfo {
                name: "repo-feature-a".to_string(),
                ..SessionInfo::default()
            },
            SessionInfo {
                name: "plain-session".to_string(),
                ..SessionInfo::default()
            },
        ]);
        state.session_manager.update_resurrectable_sessions(vec![
            (
                "repo-feature-a".to_string(),
                std::time::Duration::from_secs(1),
            ),
            ("plain-dead".to_string(), std::time::Duration::from_secs(1)),
        ]);

        let managed_sessions = registry.list().unwrap();
        state.filter_managed_sessions(&managed_sessions);

        let items = state.display_items();

        assert!(!items.iter().any(|item| matches!(
            item,
            SessionItem::ExistingSession { name, .. } if name == "repo-feature-a"
        )));
        assert!(!items.iter().any(|item| matches!(
            item,
            SessionItem::ResurrectableSession { name, .. } if name == "repo-feature-a"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            SessionItem::ExistingSession { name, .. } if name == "plain-session"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            SessionItem::ResurrectableSession { name, .. } if name == "plain-dead"
        )));
    }

    #[test]
    fn directory_session_prefers_exact_resurrectable_match() {
        let mut state = State::default();
        state.session_manager.update_resurrectable_sessions(vec![
            ("repo".to_string(), std::time::Duration::from_secs(1)),
            ("repo.2".to_string(), std::time::Duration::from_secs(1)),
        ]);

        state.create_directory_session("/tmp/repo".to_string(), "repo".to_string(), true);

        assert!(matches!(state.status, Status::Loading));
    }

    #[test]
    fn esc_clears_error_state() {
        let mut state = State::default();
        state.status = Status::Error("boom".to_string());
        state.sessions_loaded = true;
        state.directories_loaded = true;

        let handled = state.handle_key(KeyWithModifier {
            bare_key: BareKey::Esc,
            key_modifiers: BTreeSet::new(),
        });

        assert!(handled);
        assert!(matches!(state.status, Status::Ready));
    }

    #[test]
    fn waits_for_sessions_and_directories_before_ready() {
        let mut state = State::default();

        state.directories_loaded = true;
        state.sync_status();
        assert!(
            matches!(state.status, Status::Busy(ref message) if message == "Loading sessions...")
        );

        state.directories_loaded = false;
        state.sessions_loaded = true;
        state.sync_status();
        assert!(
            matches!(state.status, Status::Busy(ref message) if message == "Loading zoxide directories...")
        );

        state.directories_loaded = true;
        state.sync_status();
        assert!(matches!(state.status, Status::Ready));
    }
}
