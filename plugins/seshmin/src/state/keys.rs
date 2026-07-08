use zellij_tile::prelude::*;

use super::{ActiveScreen, DraftSession, State, Status};
use crate::session::validate_session_name;
use crate::session::SessionItem;
use crate::zoxide;

impl State {
    pub(super) fn handle_key(&mut self, key: KeyWithModifier) -> bool {
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
                let items = self.base_display_items();
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
                self.sessions_loaded = false;
                self.directories_loaded = false;
                self.status = Status::Busy("Refreshing sessions and directories...".to_string());
                self.fetch_zoxide_directories();
                true
            }
            BareKey::Char(character) if key.has_no_modifiers() && !character.is_control() => {
                let items = self.base_display_items();
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

    pub(super) fn handle_enter_on_main(&mut self) {
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

    pub(super) fn create_directory_session(
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

    pub(super) fn delete_selected_item(&mut self) {
        let Some(item) = self.selected_item_for_delete() else {
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
}
