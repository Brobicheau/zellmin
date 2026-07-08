mod directory;
mod search;

use crate::config::Config;

pub use directory::ZoxideDirectory;
pub use search::SearchEngine;

pub const MAX_SESSION_NAME_LEN: usize = 24;

pub fn parse_directories(output: &str, config: &Config) -> Vec<ZoxideDirectory> {
    let mut directories = output
        .lines()
        .filter_map(|line| parse_directory(line, config))
        .filter(|directory| is_searched_directory(&directory.directory, &config.search_directories))
        .filter(|directory| {
            !is_ignored_directory(&directory.directory, &config.ignored_directories)
        })
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
    let normalized_paths = directories
        .iter()
        .map(|directory| normalize_path(&directory.directory, &config.base_paths))
        .collect::<Vec<_>>();
    let mut assigned_names = Vec::new();

    for (directory, normalized_path) in directories.iter_mut().zip(normalized_paths.iter()) {
        directory.session_name = generate_session_name(normalized_path, &assigned_names, config);
        assigned_names.push(directory.session_name.clone());
    }
}

fn generate_session_name(path: &str, assigned_names: &[String], config: &Config) -> String {
    let segments = path_segments(path);
    if segments.is_empty() {
        return "root".to_string();
    }

    let basename = segments.last().copied().unwrap_or("root");
    let separator = &config.session_separator;
    let basename_candidate = truncate_candidate(basename.to_string(), separator);
    if !assigned_names.contains(&basename_candidate) {
        return basename_candidate;
    }

    for context_len in 2..=segments.len() {
        let candidate = truncate_candidate(
            segments[segments.len() - context_len..].join(separator),
            separator,
        );
        if !assigned_names.contains(&candidate) {
            return candidate;
        }
    }

    truncate_candidate(segments.join(separator), separator)
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

fn is_ignored_directory(path: &str, ignored_directories: &[String]) -> bool {
    ignored_directories.iter().any(|ignored_directory| {
        let ignored_directory = ignored_directory.trim_end_matches('/');
        path == ignored_directory
            || path
                .strip_prefix(ignored_directory)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn is_searched_directory(path: &str, search_directories: &[String]) -> bool {
    if search_directories.is_empty() {
        return true;
    }

    search_directories.iter().any(|search_directory| {
        let search_directory = search_directory.trim_end_matches('/');
        path == search_directory
            || path
                .strip_prefix(search_directory)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
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

        assert_eq!(directories[0].session_name, "app");
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
        assert_eq!(directories[1].session_name, "backend");
    }

    #[test]
    fn filters_ignored_directories_and_descendants() {
        let config = Config {
            ignored_directories: vec!["/home/user/projects/archive".to_string()],
            ..Config::default()
        };

        let directories = parse_directories(
            "10 /home/user/projects/app\n9 /home/user/projects/archive\n8 /home/user/projects/archive/old\n",
            &config,
        );

        assert_eq!(directories.len(), 1);
        assert_eq!(directories[0].directory, "/home/user/projects/app");
    }

    #[test]
    fn filters_to_search_directories_and_descendants() {
        let config = Config {
            search_directories: vec!["/home/user/projects".to_string()],
            ..Config::default()
        };

        let directories = parse_directories(
            "10 /home/user/projects/app\n9 /home/user/projects/api/backend\n8 /home/user/other\n",
            &config,
        );

        assert_eq!(directories.len(), 2);
        assert_eq!(directories[0].directory, "/home/user/projects/app");
        assert_eq!(directories[1].directory, "/home/user/projects/api/backend");
    }

    #[test]
    fn generated_names_stay_within_socket_safe_limit() {
        let config = Config::default();

        let directories = parse_directories(
            "10 /home/user/projects/this-is-a-very-long-directory-name/backend/service-api\n",
            &config,
        );

        assert!(directories[0].session_name.len() <= MAX_SESSION_NAME_LEN);
    }
}
