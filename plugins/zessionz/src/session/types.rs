#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionItem {
    ExistingSession {
        name: String,
        directory: String,
        is_current: bool,
    },
    ResurrectableSession {
        name: String,
        duration_text: String,
    },
    Directory {
        path: String,
        session_name: String,
    },
}

impl SessionItem {
    pub fn is_existing_session(&self) -> bool {
        matches!(self, Self::ExistingSession { .. })
    }

    pub fn label(&self) -> &str {
        match self {
            Self::ExistingSession { name, .. } => name,
            Self::ResurrectableSession { name, .. } => name,
            Self::Directory { path, .. } => path,
        }
    }
}
