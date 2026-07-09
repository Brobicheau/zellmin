mod display;
mod events;
mod keys;

use crate::config::Config;
use crate::session::SessionManager;
use crate::zoxide::{SearchEngine, ZoxideDirectory};
use zellij_tile::prelude::LayoutInfo;

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
    sessions_loaded: bool,
    directories_loaded: bool,
    pub(crate) session_name: Option<String>,
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
            show_help: false,
            sessions_loaded: false,
            directories_loaded: false,
            session_name: Option::None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;
