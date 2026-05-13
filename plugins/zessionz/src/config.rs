use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub default_layout: Option<String>,
    pub session_separator: String,
    pub show_resurrectable_sessions: bool,
    pub base_paths: Vec<String>,
    pub search_directories: Vec<String>,
    pub ignored_directories: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_layout: None,
            session_separator: ".".to_string(),
            show_resurrectable_sessions: false,
            base_paths: Vec::new(),
            search_directories: Vec::new(),
            ignored_directories: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_kdl(configuration: BTreeMap<String, String>) -> Self {
        Self {
            default_layout: configuration
                .get("default_layout")
                .and_then(|value| trim_to_option(value)),
            session_separator: configuration
                .get("session_separator")
                .and_then(|value| trim_to_option(value))
                .unwrap_or_else(|| ".".to_string()),
            show_resurrectable_sessions: configuration
                .get("show_resurrectable_sessions")
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("true")),
            base_paths: configuration
                .get("base_paths")
                .map(|value| {
                    value
                        .split('|')
                        .filter_map(trim_to_option)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            search_directories: configuration
                .get("search_directories")
                .map(|value| {
                    value
                        .split('|')
                        .filter_map(trim_to_option)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            ignored_directories: configuration
                .get("ignored_directories")
                .map(|value| {
                    value
                        .split('|')
                        .filter_map(trim_to_option)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        }
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_expected_behavior() {
        let config = Config::default();

        assert_eq!(config.default_layout, None);
        assert_eq!(config.session_separator, ".");
        assert!(!config.show_resurrectable_sessions);
        assert!(config.base_paths.is_empty());
        assert!(config.search_directories.is_empty());
        assert!(config.ignored_directories.is_empty());
    }

    #[test]
    fn parses_kdl_values() {
        let config = Config::from_kdl(BTreeMap::from([
            ("default_layout".to_string(), "dev".to_string()),
            ("session_separator".to_string(), "_".to_string()),
            (
                "show_resurrectable_sessions".to_string(),
                "true".to_string(),
            ),
            (
                "base_paths".to_string(),
                "/home/user/projects| /tmp/work ".to_string(),
            ),
            (
                "search_directories".to_string(),
                "/home/user/projects| /tmp/work ".to_string(),
            ),
            (
                "ignored_directories".to_string(),
                "/home/user/projects/archive| /tmp/scratch ".to_string(),
            ),
        ]));

        assert_eq!(config.default_layout.as_deref(), Some("dev"));
        assert_eq!(config.session_separator, "_");
        assert!(config.show_resurrectable_sessions);
        assert_eq!(config.base_paths, vec!["/home/user/projects", "/tmp/work"]);
        assert_eq!(config.search_directories, vec!["/home/user/projects", "/tmp/work"]);
        assert_eq!(
            config.ignored_directories,
            vec!["/home/user/projects/archive", "/tmp/scratch"]
        );
    }
}
