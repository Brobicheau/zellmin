use std::path::Path;

use crate::config::Config;
use crate::state::{Status, WorktreeSessionEntry};
use plugin_ui::{
    style, BoxPanel, PanelLayout, BLUE, BOLD, CYAN, DIM, GREEN, MAGENTA, RED, RESET, WHITE, YELLOW,
};

const TOP_PADDING_LINES: usize = 2;

pub fn render(
    status: &Status,
    repo_root: Option<&Path>,
    repo_name: Option<&String>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: Option<usize>,
    show_help: bool,
    cols: usize,
) {
    let title_panel = BoxPanel::new(PanelLayout::new(cols));
    print_top_padding();

    match status {
        Status::Loading => title_panel.print_status("⟳", YELLOW, "Waiting for permissions..."),
        Status::Busy(message) => title_panel.print_status("⟳", BLUE, message),
        Status::Error(message) => title_panel.print_status("✗", RED, &format!("Error: {message}")),
        Status::Success(message) => title_panel.print_status("✓", GREEN, message),
        Status::Ready => render_ready(
            repo_root,
            repo_name,
            config,
            branch_input,
            worktree_sessions,
            selected_index,
            show_help,
            title_panel,
        ),
    }
}

fn render_ready(
    repo_root: Option<&Path>,
    repo_name: Option<&String>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: Option<usize>,
    show_help: bool,
    panel: BoxPanel,
) {
    let repo_root_display = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Detecting repository root...".to_string());

    let session_format = if let Some(prefix) = &config.session_prefix {
        format!("{}-<repo>-<branch>-<hash>", prefix)
    } else {
        format!("{}|<worktree>", repo_name.unwrap().as_str())
    };

    println!();
    panel.print_top();
    panel.print_section_header("Configuration");
    panel.print_key_value("Repo", &repo_root_display);
    panel.print_key_value(
        "Worktree base",
        &format!(
            "<repo>/{}/{}",
            config.worktree_dir_name,
            pattern_display(&config.worktree_naming_pattern)
        ),
    );
    panel.print_key_value("Session name", &session_format);

    if let Some(base_branch) = &config.base_branch {
        panel.print_key_value("Base branch", base_branch);
    }

    if config.auto_fetch {
        panel.print_key_value(
            "Auto-fetch",
            &format!("enabled (remote: {})", config.remote),
        );
    }

    if !config.truncate_session_names {
        panel.print_key_value(
            "Session truncation",
            "disabled (may hit socket path limits)",
        );
    }

    panel.print_section_header("Create Worktree");
    panel.print_line(&style("Press Ctrl+H to show help", DIM, Some(WHITE)));
    let branch_value = if branch_input.is_empty() {
        style("type a branch name", DIM, Some(WHITE))
    } else {
        style(branch_input, BOLD, Some(CYAN))
    };
    let branch_line = format!(
        "{} {}",
        style("⎇ Branch", BOLD, Some(MAGENTA)),
        branch_value
    );
    panel.print_line(&branch_line);

    panel.print_section_header("Worktree Sessions");
    if worktree_sessions.is_empty() {
        panel.print_line(&style(
            "No git worktrees found for this repository.",
            DIM,
            Some(WHITE),
        ));
    } else {
        for (index, entry) in worktree_sessions.iter().enumerate() {
            let selected_marker = if selected_index == Some(index) {
                style("→", BOLD, Some(CYAN))
            } else {
                style(" ", RESET, None)
            };
            let current_marker = if entry.is_current {
                format!(" {}", style("★ current", BOLD, Some(GREEN)))
            } else {
                String::new()
            };
            let location = worktree_display_name(entry.path.as_deref(), repo_root, config);
            let line = format!(
                "{} {} {} {}{}",
                selected_marker,
                style("⎇", BOLD, Some(MAGENTA)),
                style(&entry.branch, BOLD, Some(WHITE)),
                style(&format!("[{}]", entry.session_name), DIM, Some(BLUE)),
                if current_marker.is_empty() {
                    format!(" {}", location)
                } else {
                    format!(" {}{}", location, current_marker)
                }
            );
            panel.print_line(&line);
        }
    }

    panel.print_bottom();
    println!();

    if show_help {
        panel.print_top();
        panel.print_help("Enter", "create or switch");
        panel.print_help("Up/Down", "move selection");
        panel.print_help("Ctrl+D", "delete session");
        panel.print_help("Ctrl+H", "toggle help");
        panel.print_help("Esc", "clear input or close");
        panel.print_bottom();
    }
}

fn print_top_padding() {
    for _ in 0..TOP_PADDING_LINES {
        println!();
    }
}

fn worktree_display_name(path: Option<&Path>, repo_root: Option<&Path>, config: &Config) -> String {
    let Some(path) = path else {
        return "session only".to_string();
    };

    if repo_root == Some(path) {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("main")
            .to_string();
    }

    if let Some(repo_root) = repo_root {
        let worktree_base = repo_root.join(&config.worktree_dir_name);
        if let Ok(relative_path) = path.strip_prefix(&worktree_base) {
            return relative_path.display().to_string();
        }
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("worktree")
        .to_string()
}

fn pattern_display(pattern: &crate::config::WorktreeNamingPattern) -> &str {
    match pattern {
        crate::config::WorktreeNamingPattern::Branch => "<branch>",
        crate::config::WorktreeNamingPattern::Hash => "<hash>",
        crate::config::WorktreeNamingPattern::BranchHash => "<branch>-<hash>",
    }
}
