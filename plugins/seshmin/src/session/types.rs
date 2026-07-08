#[derive(Debug, Clone, PartialEq)]
pub enum SessionItem {
    ExistingSession {
        name: String,
        directory: String,
        is_current: bool,
        is_directory_session: bool,
        zoxide_ranking: Option<f64>,
    },
    ResurrectableSession {
        name: String,
        duration_text: String,
        is_directory_session: bool,
        zoxide_ranking: Option<f64>,
    },
    Directory {
        path: String,
        session_name: String,
        zoxide_ranking: f64,
    },
}

impl SessionItem {
    pub fn is_selectable(&self) -> bool {
        !matches!(
            self,
            Self::ExistingSession {
                is_current: true,
                ..
            }
        )
    }

    pub fn is_zoxide_item(&self) -> bool {
        match self {
            Self::ExistingSession {
                is_directory_session,
                ..
            }
            | Self::ResurrectableSession {
                is_directory_session,
                ..
            } => *is_directory_session,
            Self::Directory { .. } => true,
        }
    }

    pub fn sort_group(&self) -> u8 {
        match self {
            Self::ExistingSession {
                is_current: true, ..
            } => 0,
            Self::ExistingSession {
                is_directory_session,
                ..
            } => {
                if *is_directory_session {
                    1
                } else {
                    2
                }
            }
            Self::ResurrectableSession { .. } | Self::Directory { .. } => 3,
        }
    }

    pub fn zoxide_ranking(&self) -> Option<f64> {
        match self {
            Self::ExistingSession { zoxide_ranking, .. }
            | Self::ResurrectableSession { zoxide_ranking, .. } => *zoxide_ranking,
            Self::Directory { zoxide_ranking, .. } => Some(*zoxide_ranking),
        }
    }

    pub fn compare_for_display(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_group()
            .cmp(&other.sort_group())
            .then_with(|| match (self.zoxide_ranking(), other.zoxide_ranking()) {
                (Some(left_rank), Some(right_rank)) => right_rank
                    .partial_cmp(&left_rank)
                    .unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| self.label().cmp(other.label()))
    }

    pub fn label(&self) -> &str {
        match self {
            Self::ExistingSession { name, .. } => name,
            Self::ResurrectableSession { name, .. } => name,
            Self::Directory { path, .. } => path,
        }
    }
}
