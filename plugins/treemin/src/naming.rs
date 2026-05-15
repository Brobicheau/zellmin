use crate::config::{Config, WorktreeNamingPattern};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Zellij's IPC socket path can be as short as 103 bytes on macOS, and the
// socket directory prefix itself can already consume most of that budget.
// Keep generated session names conservative so they remain usable without
// requiring users to override ZELLIJ_SOCKET_DIR.
const MAX_SESSION_NAME_LEN: usize = 24;

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

pub fn session_name(
    repo_name: Option<&str>,
    branch: &str,
    sibling_branches: &[String],
    is_main_worktree: bool,
) -> String {
    if is_main_worktree {
        return main_worktree_session_name(repo_name.unwrap_or("repo"));
    }

    assigned_session_names(repo_name, sibling_branches)
        .remove(branch)
        .unwrap_or_else(|| allocate_session_name(&linked_worktree_session_name(repo_name, branch), &BTreeSet::new()))
}

pub fn session_name_candidates(
    repo_name: Option<&str>,
    branch: &str,
    config: &Config,
    sibling_branches: &[String],
    is_main_worktree: bool,
) -> Vec<String> {
    let mut candidates = vec![session_name(
        repo_name,
        branch,
        sibling_branches,
        is_main_worktree,
    )];
    append_legacy_session_name_candidates(&mut candidates, repo_name, branch, config, is_main_worktree);

    candidates
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
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' | '|' => character,
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

fn join_session_parts(
    prefix: Option<&str>,
    repo_segment: &str,
    branch_segment: &str,
    branch_hash: &str,
) -> String {
    if let Some(prefix) = prefix {
        format!("{prefix}-{repo_segment}-{branch_segment}-{branch_hash}")
    } else {
        format!("{repo_segment}-{branch_segment}-{branch_hash}")
    }
}

fn bounded_or_unbounded_session_name(
    prefix: Option<&str>,
    repo_segment: &str,
    branch_segment: &str,
    branch_hash: &str,
    max_len: Option<usize>,
) -> String {
    let candidate = join_session_parts(prefix, repo_segment, branch_segment, branch_hash);
    if max_len.is_none_or(|max_len| candidate.len() <= max_len) {
        candidate
    } else {
        bounded_session_name(
            prefix,
            repo_segment,
            branch_segment,
            branch_hash,
            max_len.expect("checked above"),
        )
    }
}

fn bounded_session_name(
    prefix: Option<&str>,
    repo_segment: &str,
    branch_segment: &str,
    branch_hash: &str,
    max_len: usize,
) -> String {
    let reserved_len = branch_hash.len() + separator_count(prefix.is_some());
    let available = max_len.saturating_sub(reserved_len);

    let prefix_weight = usize::from(prefix.is_some());
    let total_weight = prefix_weight + 2;
    let mut prefix_budget = if prefix_weight == 0 {
        0
    } else {
        available / total_weight
    };
    let mut repo_budget = available / total_weight;
    let mut branch_budget = available.saturating_sub(prefix_budget + repo_budget);

    if let Some(prefix) = prefix {
        if prefix.len() < prefix_budget {
            let spare = prefix_budget - prefix.len();
            prefix_budget = prefix.len();
            branch_budget += spare;
        }
    }

    if repo_segment.len() < repo_budget {
        let spare = repo_budget - repo_segment.len();
        repo_budget = repo_segment.len();
        branch_budget += spare;
    }

    let prefix = prefix.map(|value| truncate_session_segment(value, prefix_budget));
    let repo_segment = truncate_session_segment(repo_segment, repo_budget);
    let branch_segment = truncate_session_segment(branch_segment, branch_budget);

    join_session_parts(
        prefix.as_deref(),
        &repo_segment,
        &branch_segment,
        branch_hash,
    )
}

fn push_unique(values: &mut Vec<String>, candidate: String) {
    if !values.iter().any(|value| value == &candidate) {
        values.push(candidate);
    }
}

fn assigned_session_names(repo_name: Option<&str>, branches: &[String]) -> BTreeMap<String, String> {
    let mut sorted_branches = branches.to_vec();
    sorted_branches.sort();
    sorted_branches.dedup();

    let mut used_names = BTreeSet::new();
    let mut assigned = BTreeMap::new();
    for branch in sorted_branches {
        let session_name = allocate_session_name(&linked_worktree_session_name(repo_name, &branch), &used_names);
        used_names.insert(session_name.clone());
        assigned.insert(branch, session_name);
    }
    assigned
}

fn linked_worktree_session_name(repo_name: Option<&str>, branch: &str) -> String {
    let branch_segment = sanitize_session_segment(branch);
    match repo_name {
        Some(repo_name) => format!("{}|{branch_segment}", sanitize_session_segment(repo_name)),
        None => branch_segment,
    }
}

fn legacy_linked_worktree_session_name(repo_name: Option<&str>, branch: &str) -> String {
    let branch_segment = sanitize_session_segment(branch);
    match repo_name {
        Some(repo_name) => format!("{}-{branch_segment}", sanitize_session_segment(repo_name)),
        None => branch_segment,
    }
}

fn allocate_session_name(branch: &str, used_names: &BTreeSet<String>) -> String {
    let sanitized = sanitize_session_segment(branch);
    let candidate = truncate_to_length(&sanitized, MAX_SESSION_NAME_LEN);
    if !used_names.contains(&candidate) {
        return candidate;
    }

    for counter in 2..=1000 {
        let suffix = format!(".{counter}");
        let candidate = format!(
            "{}{}",
            truncate_to_length(&sanitized, MAX_SESSION_NAME_LEN.saturating_sub(suffix.len())),
            suffix,
        );
        if !used_names.contains(&candidate) {
            return candidate;
        }
    }

    format!(
        "{}{}",
        truncate_to_length(&sanitized, MAX_SESSION_NAME_LEN.saturating_sub(9)),
        ".overflow"
    )
}

fn truncate_to_length(input: &str, max_len: usize) -> String {
    input.chars().take(max_len).collect()
}

fn append_legacy_session_name_candidates(
    candidates: &mut Vec<String>,
    repo_name: Option<&str>,
    branch: &str,
    config: &Config,
    is_main_worktree: bool,
) {
    let repo = repo_name.unwrap_or("repo");
    let repo_segment = sanitize_session_segment(repo);
    let branch_segment = sanitize_session_segment(branch);
    let prefix = config
        .session_prefix
        .as_deref()
        .map(sanitize_session_segment);

    if is_main_worktree {
        push_unique(candidates, main_worktree_session_name(repo));
    }

    // Add legacy hyphenated linked worktree name for existing sessions
    if !is_main_worktree {
        let legacy_name = allocate_session_name(
            &legacy_linked_worktree_session_name(repo_name, branch),
            &BTreeSet::new()
        );
        push_unique(candidates, legacy_name);
    }

    let hashes = [short_hash(branch), legacy_short_hash(branch)];
    for branch_hash in hashes {
        let bounded = bounded_or_unbounded_session_name(
            prefix.as_deref(),
            &repo_segment,
            &branch_segment,
            &branch_hash,
            Some(MAX_SESSION_NAME_LEN),
        );
        push_unique(candidates, bounded);

        let unbounded = bounded_or_unbounded_session_name(
            prefix.as_deref(),
            &repo_segment,
            &branch_segment,
            &branch_hash,
            None,
        );
        push_unique(candidates, unbounded);
    }
}

fn main_worktree_session_name(repo_name: &str) -> String {
    repo_name.to_string()
}

fn separator_count(has_prefix: bool) -> usize {
    if has_prefix {
        3
    } else {
        2
    }
}

pub(crate) fn truncate_session_segment(input: &str, max_len: usize) -> String {
    if input.len() <= max_len {
        return input.to_string();
    }

    if max_len == 0 {
        return String::new();
    }

    let min_hash_len = 6.min(max_len.saturating_sub(2));
    if min_hash_len == 0 {
        return input[..max_len].to_string();
    }

    let visible_len = max_len.saturating_sub(min_hash_len + 1);
    let mut shortened = String::new();
    shortened.push_str(&input[..visible_len]);
    shortened.push('-');
    shortened.push_str(&short_hash(input)[..min_hash_len]);
    shortened
}

fn short_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    format!("{:08x}", hash as u32)
}

fn legacy_short_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_short_session_names_unchanged() {
        assert_eq!(
            session_name(Some("repo"), "feature/test", &["feature/test".to_string()], false),
            "repo|feature-test"
        );
    }

    #[test]
    fn includes_branch_only_and_legacy_session_name_variants() {
        let config = Config::default();

        let candidates = session_name_candidates(
            Some("treemin"),
            "test/tree",
            &config,
            &["test/tree".to_string()],
            false,
        );

        assert_eq!(candidates.first().map(String::as_str), Some("treemin|test-tree"));
        assert!(candidates.iter().any(|candidate| candidate == "treemin-test-tree"));
        assert!(candidates.iter().any(|candidate| candidate == "treemin-test-tree-377d9d196e84c82"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate == "z-2dfa96-t-a4b6eb-96e84c82"));
    }

    #[test]
    fn shortens_long_branch_only_session_names_to_safe_length() {
        let session = session_name(
            Some("repo"),
            "feature/with-a-very-long-branch-name-that-would-also-overflow",
            &["feature/with-a-very-long-branch-name-that-would-also-overflow".to_string()],
            false,
        );

        assert!(session.len() <= MAX_SESSION_NAME_LEN);
        assert!(session.starts_with("repo|feature-with-a-ver"));
    }

    #[test]
    fn adds_numeric_suffix_when_branch_only_name_collides() {
        let branches = vec!["feature/test".to_string(), "feature-test".to_string()];

        assert_eq!(
            session_name(Some("repo"), "feature/test", &branches, false),
            "repo|feature-test"
        );
        assert_eq!(
            session_name(Some("repo"), "feature-test", &branches, false),
            "repo|feature-test.2"
        );
    }

    #[test]
    fn collision_suffix_skips_taken_natural_branch_name() {
        let branches = vec![
            "feature/test".to_string(),
            "feature-test".to_string(),
            "feature-test.2".to_string(),
        ];

        assert_eq!(
            session_name(Some("repo"), "feature-test.2", &branches, false),
            "repo|feature-test.2"
        );
        assert_eq!(
            session_name(Some("repo"), "feature-test", &branches, false),
            "repo|feature-test.3"
        );
    }

    #[test]
    fn uses_repo_name_for_main_worktree_session() {
        let config = Config::default();
        let candidates = session_name_candidates(
            Some("repo"),
            "main",
            &config,
            &["main".to_string()],
            true,
        );

        assert_eq!(candidates.first().map(String::as_str), Some("repo"));
        assert!(candidates.iter().any(|candidate| candidate == "repo-main-17c9aaa7"));
    }
}
