use std::time::Duration;

use zellij_tile::prelude::{LayoutInfo, SessionInfo};

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

    pub fn generate_incremented_name(&self, base_name: &str, separator: &str) -> String {
        if !self.name_exists(base_name) {
            return base_name.to_string();
        }

        for counter in 2..=1000 {
            let candidate = format!("{base_name}{separator}{counter}");
            if !self.name_exists(&candidate) {
                return candidate;
            }
        }

        format!("{base_name}{separator}overflow")
    }

    fn name_exists(&self, candidate: &str) -> bool {
        self.sessions.iter().any(|session| session.name == candidate)
            || self
                .resurrectable_sessions
                .iter()
                .any(|(name, _)| name == candidate)
    }
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

        assert_eq!(manager.generate_incremented_name("api", "."), "api.3");
    }
}
