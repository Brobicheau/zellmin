use crate::session::SessionItem;
use crate::state::{ActiveScreen, DraftSession, State, Status};
use plugin_ui::{
    style, BoxPanel, PanelLayout, BOLD, BLUE, CYAN, DIM, GREEN, MAGENTA, RED, WHITE, YELLOW,
};

pub fn render(state: &State, rows: usize, cols: usize) {
    let title_panel = BoxPanel::new(PanelLayout::new(cols));

    title_panel.print_top();
    title_panel.print_centered_line(&style("zessionz", BOLD, Some(WHITE)));
    title_panel.print_bottom();
    println!();

    match &state.status {
        Status::Loading => title_panel.print_status("⟳", YELLOW, "Waiting for permissions..."),
        Status::Busy(message) => title_panel.print_status("⟳", BLUE, message),
        Status::Error(message) => {
            title_panel.print_status("✗", RED, &format!("Error: {message}"))
        }
        Status::Ready => match state.active_screen {
            ActiveScreen::Main => render_main(state, rows, title_panel),
            ActiveScreen::NewSession => render_new_session(state, rows, title_panel),
        },
    }
}

fn render_main(state: &State, rows: usize, panel: BoxPanel) {
    let items = state.display_items();
    let visible_items = visible_slice(&items, state.selected_index(), rows.saturating_sub(16));

    panel.print_top();
    panel.print_section_header("Search");
    let search_value = if state.search_term().is_empty() {
        style("type to search", DIM, Some(WHITE))
    } else {
        style(state.search_term(), BOLD, Some(CYAN))
    };
    panel.print_line(&format!("{} {}", style("⌕", BOLD, Some(MAGENTA)), search_value));
    panel.print_key_value("Directories", &state.directory_count().to_string());
    panel.print_key_value("Sessions", &state.session_count().to_string());

    panel.print_section_header("Results");
    if items.is_empty() {
        panel.print_line(&style(
            "No zoxide directories matched the current session set.",
            DIM,
            Some(WHITE),
        ));
    } else {
        for (index, item) in visible_items {
            let selected = index == state.selected_index();
            panel.print_line(&render_item_line(item, selected, panel.width()));
        }
    }
    panel.print_bottom();
    println!();

    panel.print_top();
    panel.print_help("Enter", "switch or prepare session");
    panel.print_help("Ctrl+Enter", "quick create selected directory");
    panel.print_help("Up/Down", "move selection");
    panel.print_help("Ctrl+R", "refresh zoxide directories");
    panel.print_help("Esc", "clear search or close");
    panel.print_bottom();
}

fn render_new_session(state: &State, rows: usize, panel: BoxPanel) {
    let draft = state.draft_session.as_ref().expect("draft session must exist");
    let layout_count = draft.layout_count() + 1;
    let visible_layouts = visible_layout_slice(draft, rows.saturating_sub(16), layout_count);

    panel.print_top();
    panel.print_section_header("Create Session");
    panel.print_key_value("Folder", &draft.directory);
    panel.print_key_value("Session", &draft.session_name);
    let default_layout = state
        .config
        .default_layout
        .as_deref()
        .unwrap_or("none");
    panel.print_key_value("Default layout", default_layout);

    panel.print_section_header("Layouts");
    for (index, label, selected) in visible_layouts {
        let marker = if selected {
            style("→", BOLD, Some(CYAN))
        } else {
            " ".to_string()
        };
        let color = if index == 0 { Some(MAGENTA) } else { Some(WHITE) };
        panel.print_line(&format!("{} {}", marker, style(&label, BOLD, color)));
    }
    panel.print_bottom();
    println!();

    panel.print_top();
    panel.print_help("Type", "edit session name");
    panel.print_help("Enter", "create with selected layout");
    panel.print_help("Ctrl+Enter", "create with default layout");
    panel.print_help("Up/Down", "move layout selection");
    panel.print_help("Esc", "back to results");
    panel.print_bottom();
}

fn render_item_line(item: &SessionItem, selected: bool, width: usize) -> String {
    let selected_marker = if selected {
        style("→", BOLD, Some(CYAN))
    } else {
        " ".to_string()
    };

    match item {
        SessionItem::ExistingSession {
            name,
            directory,
            is_current,
        } => {
            let current = if *is_current {
                format!(" {}", style("★ current", BOLD, Some(GREEN)))
            } else {
                String::new()
            };
            format!(
                "{} {} {}{}",
                selected_marker,
                style("●", BOLD, Some(GREEN)),
                style(name, BOLD, Some(WHITE)),
                style(&format!(" ({directory}){current}"), DIM, Some(BLUE))
            )
        }
        SessionItem::ResurrectableSession {
            name,
            duration_text,
        } => format!(
            "{} {} {} {}",
            selected_marker,
            style("↺", BOLD, Some(YELLOW)),
            style(name, BOLD, Some(WHITE)),
            style(duration_text, DIM, Some(BLUE))
        ),
        SessionItem::Directory { path, session_name } => format!(
            "{} {} {} {}",
            selected_marker,
            style("○", BOLD, Some(MAGENTA)),
            truncate(path, width / 2),
            style(&format!("[{session_name}]"), DIM, Some(BLUE))
        ),
    }
}

fn visible_slice<'a>(
    items: &'a [SessionItem],
    selected_index: usize,
    max_rows: usize,
) -> Vec<(usize, &'a SessionItem)> {
    if items.is_empty() {
        return Vec::new();
    }

    let max_rows = max_rows.max(3).min(items.len());
    let start = selected_index.saturating_sub(max_rows / 2).min(items.len() - max_rows);
    items
        .iter()
        .enumerate()
        .skip(start)
        .take(max_rows)
        .collect()
}

fn visible_layout_slice(
    draft: &DraftSession,
    max_rows: usize,
    layout_count: usize,
) -> Vec<(usize, String, bool)> {
    let options = std::iter::once("No layout".to_string())
        .chain(draft.layouts.iter().map(|layout| layout.name().to_string()))
        .collect::<Vec<_>>();
    let max_rows = max_rows.max(3).min(layout_count);
    let start = draft
        .selected_layout_index
        .saturating_sub(max_rows / 2)
        .min(layout_count.saturating_sub(max_rows));

    options
        .into_iter()
        .enumerate()
        .skip(start)
        .take(max_rows)
        .map(|(index, label)| (index, label, index == draft.selected_layout_index))
        .collect()
}

fn truncate(input: &str, max_len: usize) -> String {
    let input_chars = input.chars().collect::<Vec<_>>();
    if input_chars.len() <= max_len {
        input.to_string()
    } else if max_len <= 1 {
        input_chars.into_iter().take(max_len).collect()
    } else {
        format!("{}…", input_chars.into_iter().take(max_len - 1).collect::<String>())
    }
}
