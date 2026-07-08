#[derive(Debug, Clone, PartialEq)]
pub enum SessionItem {
    ExistingSession {
        name: String,
        directory: String,
        is_current: bool,
        is_directory_session: bool,
        zoxide_ranking: Option<f64>,
    },
    Directory {
        path: String,
        session_name: String,
        zoxide_ranking: f64,
    },
}

pub fn next_selectable_index<T>(
    items: &[T],
    current_index: usize,
    forward: bool,
    is_selectable: impl Fn(&T) -> bool,
) -> Option<usize> {
    if items.is_empty() {
        return None;
    }

    if forward {
        ((current_index + 1)..items.len()).find(|index| is_selectable(&items[*index]))
    } else {
        (0..current_index)
            .rev()
            .find(|index| is_selectable(&items[*index]))
    }
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
            Self::Directory { .. } => 3,
        }
    }

    pub fn zoxide_ranking(&self) -> Option<f64> {
        match self {
            Self::ExistingSession { zoxide_ranking, .. } => *zoxide_ranking,
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
            Self::Directory { path, .. } => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::next_selectable_index;

    #[test]
    fn next_selectable_index_does_not_wrap_forward() {
        let items = [true, true, true];

        assert_eq!(
            next_selectable_index(&items, 2, true, |is_selectable| *is_selectable),
            None
        );
    }

    #[test]
    fn next_selectable_index_does_not_wrap_backward() {
        let items = [true, true, true];

        assert_eq!(
            next_selectable_index(&items, 0, false, |is_selectable| *is_selectable),
            None
        );
    }

    #[test]
    fn next_selectable_index_skips_unselectable_items() {
        let items = [true, false, true];

        assert_eq!(
            next_selectable_index(&items, 0, true, |is_selectable| *is_selectable),
            Some(2)
        );
        assert_eq!(
            next_selectable_index(&items, 2, false, |is_selectable| *is_selectable),
            Some(0)
        );
    }
}
