mod config;
mod session;
mod state;
mod storage;
mod ui;
mod zoxide;

use std::collections::BTreeMap;

use state::State;
use zellij_tile::prelude::*;

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        State::load_plugin(self, configuration);
    }

    fn update(&mut self, event: Event) -> bool {
        State::update_plugin(self, event)
    }

    fn render(&mut self, rows: usize, cols: usize) {
        State::render_plugin(self, rows, cols);
    }
}
