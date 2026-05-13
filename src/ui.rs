use std::path::Path;

use crate::config::Config;
use crate::naming::{sanitize_session_segment, session_name};
use crate::state::{Status, WorktreeSessionEntry};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";

pub fn render(
    status: &Status,
    repo_root: Option<&Path>,
    repo_name: Option<&String>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: usize,
    cols: usize,
) {
    let width = panel_width(cols);
    let gutter = horizontal_gutter(cols, width);

    print_border('┌', '─', '┐', width, gutter);
    print_box_line(
        &centered_styled(&style("zitree", BOLD, Some(WHITE)), width),
        width,
        gutter,
    );
    print_border('└', '─', '┘', width, gutter);
    println!();

    match status {
        Status::Loading => print_status_panel(
            &status_line("⟳", YELLOW, "Waiting for permissions..."),
            width,
            gutter,
        ),
        Status::Busy(message) => {
            print_status_panel(&status_line("⟳", BLUE, message), width, gutter)
        }
        Status::Error(message) => print_status_panel(
            &status_line("✗", RED, &format!("Error: {message}")),
            width,
            gutter,
        ),
        Status::Success(message) => {
            print_status_panel(&status_line("✓", GREEN, message), width, gutter)
        }
        Status::Ready => render_ready(
            repo_root,
            repo_name,
            config,
            branch_input,
            worktree_sessions,
            selected_index,
            width,
            gutter,
        ),
    }
}

fn render_ready(
    repo_root: Option<&Path>,
    repo_name: Option<&String>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: usize,
    width: usize,
    gutter: usize,
) {
    let repo_root_display = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Detecting repository root...".to_string());

    let name = session_name(Some(repo_name.unwrap().as_str()), branch_input, config);

    let session_format = if let Some(prefix) = &config.session_prefix {
        format!("{}-<repo>-<branch>-<hash>", prefix)
    } else {
        format!("{}-<branch>-<hash>", name)
    };

    print_border('┌', '─', '┐', width, gutter);
    print_section_header("Configuration", width, gutter);
    print_kv_line("Repo", &repo_root_display, width, gutter);
    print_kv_line(
        "Worktree base",
        &format!(
            "<repo>/{}/{}",
            config.worktree_dir_name,
            pattern_display(&config.worktree_naming_pattern)
        ),
        width,
        gutter,
    );
    print_kv_line("Session name", &session_format, width, gutter);

    if let Some(base_branch) = &config.base_branch {
        print_kv_line("Base branch", base_branch, width, gutter);
    }

    if config.auto_fetch {
        print_kv_line(
            "Auto-fetch",
            &format!("enabled (remote: {})", config.remote),
            width,
            gutter,
        );
    }

    print_section_header("Create Worktree", width, gutter);
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
    print_box_line(&branch_line, width, gutter);

    print_section_header("Worktree Sessions", width, gutter);
    if worktree_sessions.is_empty() {
        print_box_line(
            &style(
                "No git worktrees found for this repository.",
                DIM,
                Some(WHITE),
            ),
            width,
            gutter,
        );
    } else {
        for (index, entry) in worktree_sessions.iter().enumerate() {
            let selected_marker = if index == selected_index {
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
            print_box_line(&line, width, gutter);
        }
    }

    print_border('└', '─', '┘', width, gutter);
    println!();

    print_border('┌', '─', '┐', width, gutter);
    print_help_ln("Enter", "create or switch", width, gutter);
    print_help_ln("Up/Down", "move selection", width, gutter);
    print_help_ln("Delete", "delete session", width, gutter);
    print_help_ln("Esc", "clear input", width, gutter);
    print_border('└', '─', '┘', width, gutter);
}

fn print_help_ln(key: &str, title: &str, width: usize, gutter: usize) {
    print_box_line(&format!("{} {} ", keycap(key), title), width, gutter);
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

fn panel_width(cols: usize) -> usize {
    cols.saturating_sub(4).clamp(16, 96)
}

fn print_status_panel(message: &str, width: usize, gutter: usize) {
    print_border('┌', '─', '┐', width, gutter);
    print_box_line(message, width, gutter);
    print_border('└', '─', '┘', width, gutter);
}

fn print_section_header(title: &str, width: usize, gutter: usize) {
    print_box_line("", width, gutter);
    let header = format!("{} ", style(title, BOLD, Some(WHITE)));
    let plain_len = title.chars().count() + 2;
    let fill = width.saturating_sub(plain_len + 1);
    print_box_line(&format!("{}{}", header, "─".repeat(fill)), width, gutter);
}

fn print_kv_line(label: &str, value: &str, width: usize, gutter: usize) {
    let line = format!(
        "{} {}",
        style(&format!("{}:", label), BOLD, Some(BLUE)),
        style(value, RESET, Some(WHITE))
    );
    print_box_line(&line, width, gutter);
}

fn print_border(left: char, fill: char, right: char, width: usize, gutter: usize) {
    println!(
        "{}{}{}{}",
        " ".repeat(gutter),
        left,
        fill.to_string().repeat(width + 2),
        right
    );
}

fn print_box_line(content: &str, width: usize, gutter: usize) {
    let visible_len = visible_width(content);
    let padding = width.saturating_sub(visible_len);
    println!(
        "{}│ {}{} │",
        " ".repeat(gutter),
        content,
        " ".repeat(padding)
    );
}

fn centered_styled(input: &str, width: usize) -> String {
    let visible_len = visible_width(input);
    if visible_len >= width {
        return input.to_string();
    }
    let padding = (width - visible_len) / 2;
    format!("{}{}", " ".repeat(padding), input)
}

fn keycap(label: &str) -> String {
    style(&format!("[{}]", label), BOLD, Some(CYAN))
}

fn status_line(icon: &str, color: &str, message: &str) -> String {
    format!(
        "{} {}",
        style(icon, BOLD, Some(color)),
        style(message, BOLD, Some(WHITE))
    )
}

fn style(input: &str, modifier: &str, color: Option<&str>) -> String {
    let color = color.unwrap_or("");
    format!("{}{}{}{}", modifier, color, input, RESET)
}

fn horizontal_gutter(cols: usize, width: usize) -> usize {
    cols.saturating_sub(width + 4) / 2
}

fn visible_width(input: &str) -> usize {
    let mut width = 0;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }

    width
}
