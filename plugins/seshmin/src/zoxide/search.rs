use crate::session::{next_selectable_index, SessionItem};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub item: SessionItem,
    pub score: i64,
}

pub struct SearchEngine {
    search_term: String,
    results: Vec<SearchResult>,
    selected_index: Option<usize>,
    matcher: SkimMatcherV2,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self {
            search_term: String::new(),
            results: Vec::new(),
            selected_index: None,
            matcher: SkimMatcherV2::default().use_cache(true),
        }
    }
}

impl SearchEngine {
    pub fn search_term(&self) -> &str {
        &self.search_term
    }

    pub fn is_searching(&self) -> bool {
        !self.search_term.is_empty()
    }

    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_item(&self) -> Option<&SessionItem> {
        self.selected_index
            .and_then(|index| self.results.get(index))
            .map(|result| &result.item)
    }

    pub fn add_char(&mut self, character: char, items: &[SessionItem]) {
        self.search_term.push(character);
        self.refresh(items);
    }

    pub fn backspace(&mut self, items: &[SessionItem]) {
        self.search_term.pop();
        self.refresh(items);
    }

    pub fn clear(&mut self) {
        self.search_term.clear();
        self.results.clear();
        self.selected_index = None;
    }

    pub fn refresh(&mut self, items: &[SessionItem]) {
        if self.search_term.is_empty() {
            self.results.clear();
            self.selected_index = None;
            return;
        }

        let mut results = items
            .iter()
            .filter_map(|item| {
                self.matcher
                    .fuzzy_match(&search_text(item), &self.search_term)
                    .map(|score| SearchResult {
                        item: item.clone(),
                        score,
                    })
            })
            .collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.item.compare_for_display(&right.item))
                .then_with(|| left.item.label().cmp(right.item.label()))
        });

        self.results = results;
        self.selected_index = self.first_selectable_index();
    }

    pub fn move_up(&mut self) {
        let Some(selected_index) = self.selected_index else {
            return;
        };

        self.selected_index =
            next_selectable_index(&self.results, selected_index, false, |result| {
                result.item.is_selectable()
            });
    }

    pub fn move_down(&mut self) {
        let Some(selected_index) = self.selected_index else {
            return;
        };

        self.selected_index =
            next_selectable_index(&self.results, selected_index, true, |result| {
                result.item.is_selectable()
            });
    }

    fn first_selectable_index(&self) -> Option<usize> {
        self.results
            .iter()
            .position(|result| result.item.is_selectable())
    }
}

fn search_text(item: &SessionItem) -> String {
    match item {
        SessionItem::ExistingSession {
            name,
            directory,
            is_current,
            ..
        } => {
            let current_text = if *is_current { " current" } else { "" };
            format!("{name} {directory}{current_text}")
        }
        SessionItem::ResurrectableSession {
            name,
            duration_text,
            ..
        } => format!("{name} {duration_text}"),
        SessionItem::Directory {
            path, session_name, ..
        } => format!("{path} {session_name}"),
    }
}
