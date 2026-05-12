use std::path::Path;

use crate::state::Status;

pub fn render(status: &Status, repo_root: Option<&Path>, worktree_dir_name: &str, branch_input: &str, cols: usize) {
    let title = centered("zitree", cols);
    println!("{title}");
    println!();

    match status {
        Status::Loading => println!("Waiting for permissions..."),
        Status::Busy(message) => println!("{message}"),
        Status::Error(message) => println!("Error: {message}"),
        Status::Success(message) => println!("{message}"),
        Status::Ready => render_ready(repo_root, worktree_dir_name, branch_input),
    }
}

fn render_ready(repo_root: Option<&Path>, worktree_dir_name: &str, branch_input: &str) {
    let repo_root = repo_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Detecting repository root...".to_string());
    println!("Repo: {repo_root}");
    println!("Worktree base: <repo>/{worktree_dir_name}/<branch>");
    println!("Session name: <repo>-<branch>");
    println!();
    println!("> Branch: {branch_input}");
    println!();
    println!("Enter create worktree and switch session");
    println!("Esc clear input");
    println!("Backspace delete character");
    println!("Ctrl-c does nothing inside plugin pane");
}

fn centered(input: &str, cols: usize) -> String {
    if input.len() >= cols {
        return input.to_string();
    }
    let padding = (cols - input.len()) / 2;
    format!("{}{}", " ".repeat(padding), input)
}
