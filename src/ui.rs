use std::path::Path;

use crate::config::Config;
use crate::state::{Status, WorktreeSessionEntry};

pub fn render(
    status: &Status,
    repo_root: Option<&Path>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: usize,
    cols: usize,
) {
    let title = centered("zitree", cols);
    println!("{title}");
    println!();

    match status {
        Status::Loading => println!("Waiting for permissions..."),
        Status::Busy(message) => println!("{message}"),
        Status::Error(message) => println!("Error: {message}"),
        Status::Success(message) => println!("{message}"),
        Status::Ready => render_ready(
            repo_root,
            config,
            branch_input,
            worktree_sessions,
            selected_index,
        ),
    }
}

fn render_ready(
    repo_root: Option<&Path>,
    config: &Config,
    branch_input: &str,
    worktree_sessions: &[WorktreeSessionEntry],
    selected_index: usize,
) {
    let repo_root = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Detecting repository root...".to_string());
    println!("Repo: {repo_root}");
    println!(
        "Worktree base: <repo>/{}/{}",
        config.worktree_dir_name,
        pattern_display(&config.worktree_naming_pattern)
    );

    let session_format = if let Some(prefix) = &config.session_prefix {
        format!("{}-<repo>-<branch>-<hash>", prefix)
    } else {
        "<repo>-<branch>-<hash>".to_string()
    };
    println!("Session name: {}", session_format);

    if let Some(base_branch) = &config.base_branch {
        println!("Base branch: {}", base_branch);
    }

    if config.auto_fetch {
        println!("Auto-fetch: enabled (remote: {})", config.remote);
    }

    println!();
    println!("> Branch: {branch_input}");
    println!();
    println!("Worktree sessions:");
    if worktree_sessions.is_empty() {
        println!("  No live worktree sessions found.");
    } else {
        for (index, entry) in worktree_sessions.iter().enumerate() {
            let selected_marker = if index == selected_index { ">" } else { " " };
            let current_marker = if entry.is_current { " current" } else { "" };
            let location = entry
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "session only".to_string());
            println!(
                "{selected_marker} {} [{}] {}{}",
                entry.branch, entry.session_name, location, current_marker
            );
        }
    }
    println!();
    println!("Enter create worktree and switch session when branch is set");
    println!("Enter switch selected worktree session when branch is empty");
    println!("Up/Down select worktree session");
    println!("Esc clear input");
    println!("Backspace delete character");
    println!("Ctrl-c does nothing inside plugin pane");
}

fn pattern_display(pattern: &crate::config::WorktreeNamingPattern) -> &str {
    match pattern {
        crate::config::WorktreeNamingPattern::Branch => "<branch>",
        crate::config::WorktreeNamingPattern::Hash => "<hash>",
        crate::config::WorktreeNamingPattern::BranchHash => "<branch>-<hash>",
    }
}

fn centered(input: &str, cols: usize) -> String {
    if input.len() >= cols {
        return input.to_string();
    }
    let padding = (cols - input.len()) / 2;
    format!("{}{}", " ".repeat(padding), input)
}
