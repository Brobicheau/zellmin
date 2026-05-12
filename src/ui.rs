use std::path::Path;

use crate::config::Config;
use crate::state::Status;

pub fn render(status: &Status, repo_root: Option<&Path>, config: &Config, branch_input: &str, cols: usize) {
    let title = centered("zitree", cols);
    println!("{title}");
    println!();

    match status {
        Status::Loading => println!("Waiting for permissions..."),
        Status::Busy(message) => println!("{message}"),
        Status::Error(message) => println!("Error: {message}"),
        Status::Success(message) => println!("{message}"),
        Status::Ready => render_ready(repo_root, config, branch_input),
    }
}

fn render_ready(repo_root: Option<&Path>, config: &Config, branch_input: &str) {
    let repo_root = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Detecting repository root...".to_string());
    println!("Repo: {repo_root}");
    println!("Worktree base: <repo>/{}/{}", config.worktree_dir_name, pattern_display(&config.worktree_naming_pattern));
    
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
    println!("Enter create worktree and switch session");
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
