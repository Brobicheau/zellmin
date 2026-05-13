use std::time::Duration;

use zellij_tile::prelude::{delete_dead_session, kill_sessions, LayoutInfo, SessionInfo};

#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: Vec<SessionInfo>,
    resurrectable_sessions: Vec<(String, Duration)>,
}

impl SessionManager {
    pub fn update_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
    }

    pub fn update_resurrectable_sessions(&mut self, sessions: Vec<(String, Duration)>) {
        self.resurrectable_sessions = sessions;
    }

    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    pub fn resurrectable_sessions(&self) -> &[(String, Duration)] {
        &self.resurrectable_sessions
    }

    pub fn current_session_name(&self) -> Option<&str> {
        self.sessions
            .iter()
            .find(|session| session.is_current_session)
            .map(|session| session.name.as_str())
    }

    pub fn current_layouts(&self) -> Vec<LayoutInfo> {
        self.sessions
            .iter()
            .find(|session| session.is_current_session)
            .map(|session| session.available_layouts.clone())
            .unwrap_or_default()
    }

    pub fn generate_incremented_name(
        &self,
        base_name: &str,
        separator: &str,
        max_len: usize,
    ) -> String {
        if !self.name_exists(base_name) {
            return truncate_to_length(base_name, max_len);
        }

        for counter in 2..=1000 {
            let suffix = format!("{separator}{counter}");
            let candidate = format!(
                "{}{}",
                truncate_to_length(base_name, max_len.saturating_sub(suffix.len())),
                suffix
            );
            if !self.name_exists(&candidate) {
                return candidate;
            }
        }

        let suffix = format!("{separator}overflow");
        format!(
            "{}{}",
            truncate_to_length(base_name, max_len.saturating_sub(suffix.len())),
            suffix
        )
    }

    pub fn delete_session(&self, session_name: &str) {
        if self
            .resurrectable_sessions
            .iter()
            .any(|(name, _)| name == session_name)
        {
            delete_dead_session(session_name);
        } else {
            kill_sessions(&[session_name]);
        }
    }

    pub fn resurrectable_session_name(&self, session_name: &str) -> Option<&str> {
        self.resurrectable_sessions
            .iter()
            .find(|(name, _)| name == session_name)
            .map(|(name, _)| name.as_str())
    }

    fn name_exists(&self, candidate: &str) -> bool {
        self.sessions.iter().any(|session| session.name == candidate)
            || self
                .resurrectable_sessions
                .iter()
                .any(|(name, _)| name == candidate)
    }
}

fn truncate_to_length(input: &str, max_len: usize) -> String {
    input.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_conflicting_session_names() {
        let mut manager = SessionManager::default();
        manager.update_resurrectable_sessions(vec![
            ("api".to_string(), Duration::from_secs(1)),
            ("api.2".to_string(), Duration::from_secs(1)),
        ]);

        assert_eq!(manager.generate_incremented_name("api", ".", 24), "api.3");
    }

    #[test]
    fn incremented_names_respect_max_length() {
        let mut manager = SessionManager::default();
        manager.update_resurrectable_sessions(vec![(
            "abcdefghijklmnopqrst.2".to_string(),
            Duration::from_secs(1),
        )]);

        let name = manager.generate_incremented_name("abcdefghijklmnopqrstuvwx", ".", 24);

        assert_eq!(name, "abcdefghijklmnopqrst.3");
        assert!(name.len() <= 24);
    }

    #[test]
    fn finds_exact_resurrectable_session_name() {
        let mut manager = SessionManager::default();
        manager.update_resurrectable_sessions(vec![
            ("api".to_string(), Duration::from_secs(1)),
            ("api.2".to_string(), Duration::from_secs(1)),
        ]);

        assert_eq!(manager.resurrectable_session_name("api"), Some("api"));
        assert_eq!(manager.resurrectable_session_name("missing"), None);
    }
}
