mod commands;
mod config;
mod naming;
mod state;
mod storage;
mod ui;
mod validation;

use state::State;
use zellij_tile::prelude::*;

register_plugin!(State);
