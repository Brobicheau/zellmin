mod directory;
mod search;

use std::path::Path;

use crate::config::Config;

pub use directory::ZoxideDirectory;
pub use search::SearchEngine;

const MAX_SESSION_NAME_LEN: usize = 29;

pub fn parse_directories(output: &str, config: &Config) -> Vec<ZoxideDirectory> {
    let mut directories = output
        .lines()
        .filter_map(|line| parse_directory(line, config))
        .collect::<Vec<_>>();

    assign_session_names(&mut directories, config);
    directories.sort_by(|left, right| {
        right
            .ranking
            .partial_cmp(&left.ranking)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    directories
}

fn parse_directory(line: &str, _config: &Config) -> Option<ZoxideDirectory> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (score, path) = trimmed.split_once(' ')?;
    let ranking = score.parse::<f64>().ok()?;
    Some(ZoxideDirectory {
        ranking,
        directory: path.to_string(),
        session_name: String::new(),
    })
}

fn assign_session_names(directories: &mut [ZoxideDirectory], config: &Config) {
    let all_paths = directories
        .iter()
        .map(|directory| directory.directory.as_str())
        .collect::<Vec<_>>();

    for directory in directories.iter_mut() {
        directory.session_name = generate_session_name(&directory.directory, &all_paths, config);
    }
}

fn generate_session_name(path: &str, all_paths: &[&str], config: &Config) -> String {
    let normalized_path = normalize_path(path, &config.base_paths);
    let segments = path_segments(&normalized_path);
    if segments.is_empty() {
        return "root".to_string();
    }

    let basename = segments.last().copied().unwrap_or("root");
    let separator = &config.session_separator;
    let conflicting_paths = all_paths
        .iter()
        .copied()
        .filter(|other_path| {
            let other_segments = path_segments(&normalize_path(other_path, &config.base_paths));
            other_segments.last().copied() == Some(basename)
        })
        .collect::<Vec<_>>();

    let min_segments = if is_nested_in_paths(path, all_paths) { 2 } else { 1 };
    for context_len in min_segments..=segments.len() {
        let candidate = segments[segments.len() - context_len..].join(separator);
        if conflicting_paths.iter().all(|other_path| {
            if *other_path == path {
                return true;
            }

            let other_segments = path_segments(&normalize_path(other_path, &config.base_paths));
            other_segments.len() < context_len
                || other_segments[other_segments.len() - context_len..].join(separator) != candidate
        }) {
            return truncate_candidate(candidate, separator);
        }
    }

    truncate_candidate(segments.join(separator), separator)
}

fn is_nested_in_paths(path: &str, all_paths: &[&str]) -> bool {
    let normalized_path = Path::new(path);
    all_paths.iter().copied().any(|other_path| {
        other_path != path && normalized_path.starts_with(Path::new(other_path))
    })
}

fn normalize_path(path: &str, base_paths: &[String]) -> String {
    let mut best_match = None;

    for base_path in base_paths {
        let candidate = base_path.trim_end_matches('/');
        let is_match = path == candidate
            || path
                .strip_prefix(candidate)
                .is_some_and(|suffix| suffix.starts_with('/'));
        if is_match
            && best_match
                .as_ref()
                .is_none_or(|current: &&str| candidate.len() > current.len())
        {
            best_match = Some(candidate);
        }
    }

    let Some(best_match) = best_match else {
        return path.to_string();
    };

    if path == best_match {
        return path.to_string();
    }

    path.trim_start_matches(best_match)
        .trim_start_matches('/')
        .to_string()
}

fn path_segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn truncate_candidate(candidate: String, separator: &str) -> String {
    if candidate.len() <= MAX_SESSION_NAME_LEN {
        return candidate;
    }

    let segments = candidate.split(separator).collect::<Vec<_>>();
    let abbreviated = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if index + 1 == segments.len() {
                truncate_segment(segment, 12)
            } else {
                abbreviate_segment(segment)
            }
        })
        .collect::<Vec<_>>()
        .join(separator);

    if abbreviated.len() <= MAX_SESSION_NAME_LEN {
        abbreviated
    } else {
        abbreviated[..MAX_SESSION_NAME_LEN].to_string()
    }
}

fn abbreviate_segment(segment: &str) -> String {
    if segment.len() <= 3 {
        return segment.to_string();
    }

    if segment.contains('-') || segment.contains('_') {
        return segment
            .split(&['-', '_'][..])
            .filter_map(|part| part.chars().next())
            .map(|character| character.to_string())
            .collect::<Vec<_>>()
            .join("-");
    }

    segment.chars().take(3).collect()
}

fn truncate_segment(segment: &str, max_len: usize) -> String {
    if segment.len() <= max_len {
        segment.to_string()
    } else {
        segment[..max_len].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_base_path_when_generating_names() {
        let config = Config {
            base_paths: vec!["/home/user/projects".to_string()],
            ..Config::default()
        };

        let directories = parse_directories(
            "10 /home/user/projects/client/app\n9 /home/user/projects/admin/app\n",
            &config,
        );

        assert_eq!(directories[0].session_name, "client.app");
        assert_eq!(directories[1].session_name, "admin.app");
    }

    #[test]
    fn keeps_nested_directories_contextual() {
        let config = Config::default();

        let directories = parse_directories(
            "10 /home/user/projects/api\n9 /home/user/projects/api/backend\n",
            &config,
        );

        assert_eq!(directories[0].session_name, "api");
        assert_eq!(directories[1].session_name, "api.backend");
    }
}
