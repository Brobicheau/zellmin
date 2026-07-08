use humantime::format_duration;

use super::State;
use crate::session::{next_selectable_index, SessionItem};
use crate::zoxide::ZoxideDirectory;

impl State {
    pub(crate) fn display_items(&self) -> Vec<SessionItem> {
        // The UI reads search results while searching; otherwise it renders the full picker list.
        if self.search_engine.is_searching() {
            self.search_engine
                .results()
                .iter()
                .map(|result| result.item.clone())
                .collect()
        } else {
            self.base_display_items()
        }
    }

    pub(super) fn base_display_items(&self) -> Vec<SessionItem> {
        // Search refreshes against the full list to avoid feeding results back into itself.
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

        items.sort_by(|left, right| left.compare_for_display(right));
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

    pub(super) fn refresh_search(&mut self) {
        if self.search_engine.is_searching() {
            let items = self.base_display_items();
            self.search_engine.refresh(&items);
        }
    }

    pub(super) fn selected_item(&self) -> Option<SessionItem> {
        if self.search_engine.is_searching() {
            self.search_engine.selected_item().cloned()
        } else {
            self.display_items()
                .get(self.selected_index)
                .filter(|item| item.is_selectable())
                .cloned()
        }
    }

    pub(super) fn selected_item_for_delete(&self) -> Option<SessionItem> {
        if self.search_engine.is_searching() {
            self.search_engine.selected_item().cloned()
        } else {
            self.display_items().get(self.selected_index).cloned()
        }
    }

    pub(super) fn move_selection(&mut self, forward: bool) {
        if self.search_engine.is_searching() {
            if forward {
                self.search_engine.move_down();
            } else {
                self.search_engine.move_up();
            }
            return;
        }

        let items = self.display_items();
        if let Some(index) = next_selectable_index(
            &items,
            self.selected_index,
            forward,
            SessionItem::is_selectable,
        ) {
            self.selected_index = index;
        }
    }

    pub(super) fn clamp_selection(&mut self) {
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

pub(super) fn matching_directory<'a>(
    session_name: &str,
    directories: &'a [ZoxideDirectory],
    separator: &str,
) -> Option<&'a ZoxideDirectory> {
    // Treat incremented names like `repo.2` as belonging to the original `repo` directory.
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
