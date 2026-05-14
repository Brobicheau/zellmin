mod commands;
mod config;
mod naming;
mod storage;
mod state;
mod ui;
mod validation;

use state::State;
use zellij_tile::prelude::*;

register_plugin!(State);
