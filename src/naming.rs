use std::path::{Path, PathBuf};
use crate::config::{Config, WorktreeNamingPattern};

pub fn worktree_path(repo_root: &Path, config: &Config, branch: &str) -> PathBuf {
    let worktree_name = match config.worktree_naming_pattern {
        WorktreeNamingPattern::Branch => sanitize_path_segment(branch),
        WorktreeNamingPattern::Hash => short_hash(branch),
        WorktreeNamingPattern::BranchHash => {
            format!("{}-{}", sanitize_path_segment(branch), short_hash(branch))
        }
    };
    
    repo_root
        .join(&config.worktree_dir_name)
        .join(worktree_name)
}

pub fn session_name(repo_name: Option<&str>, branch: &str, config: &Config) -> String {
    let repo = repo_name.unwrap_or("repo");
    let branch_hash = short_hash(branch);
    
    let base_name = format!(
        "{}-{}-{}",
        sanitize_session_segment(repo),
        sanitize_session_segment(branch),
        branch_hash
    );
    
    if let Some(prefix) = &config.session_prefix {
        format!("{}-{}", sanitize_session_segment(prefix), base_name)
    } else {
        base_name
    }
}

pub fn sanitize_path_segment(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|character| match character {
            '/' => '/',
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => character,
            _ => '-',
        })
        .collect();
    sanitized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(sanitize_session_segment)
        .collect::<Vec<_>>()
        .join("/")
}

pub fn sanitize_session_segment(input: &str) -> String {
    let collapsed = input
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => character,
            _ => '-',
        })
        .collect::<String>();
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "worktree".to_string()
    } else {
        trim_repeated_dashes(trimmed)
    }
}

fn trim_repeated_dashes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous_was_dash = false;
    for character in input.chars() {
        if character == '-' {
            if previous_was_dash {
                continue;
            }
            previous_was_dash = true;
        } else {
            previous_was_dash = false;
        }
        output.push(character);
    }
    output
}

fn short_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    format!("{hash:08x}")
}
