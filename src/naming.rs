use std::path::{Path, PathBuf};

pub fn worktree_path(repo_root: &Path, worktree_dir_name: &str, branch: &str) -> PathBuf {
    repo_root
        .join(worktree_dir_name)
        .join(sanitize_path_segment(branch))
}

pub fn session_name(repo_name: Option<&str>, branch: &str) -> String {
    let repo = repo_name.unwrap_or("repo");
    let branch_hash = short_hash(branch);
    format!(
        "{}-{}-{}",
        sanitize_session_segment(repo),
        sanitize_session_segment(branch),
        branch_hash
    )
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
