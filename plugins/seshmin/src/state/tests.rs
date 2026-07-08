use std::collections::BTreeSet;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::display::matching_directory;
use super::*;
use crate::session::SessionItem;
use crate::storage::test_treemin_registry;
use crate::zoxide::ZoxideDirectory;
use zellij_tile::prelude::*;

#[test]
fn matches_incremented_session_names() {
    let directories = vec![ZoxideDirectory {
        ranking: 1.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];

    assert!(matching_directory("repo.3", &directories, ".").is_some());
    assert!(matching_directory("repo-x", &directories, ".").is_none());
}

#[test]
fn enter_opens_layout_picker_only_without_default_layout() {
    let mut state = State::default();
    state.directories = vec![ZoxideDirectory {
        ranking: 1.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];

    state.handle_enter_on_main();

    assert!(state.draft_session.is_some());
    assert_eq!(state.active_screen, ActiveScreen::NewSession);
}

#[test]
fn enter_skips_layout_picker_when_default_layout_is_set() {
    let mut state = State::default();
    state.config.default_layout = Some("dev".to_string());
    state.directories = vec![ZoxideDirectory {
        ranking: 1.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];

    state.handle_enter_on_main();

    assert!(state.draft_session.is_none());
    assert_eq!(state.active_screen, ActiveScreen::Main);
}

#[test]
fn delete_requires_selected_session_not_directory() {
    let mut state = State::default();
    state.directories = vec![ZoxideDirectory {
        ranking: 1.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];

    state.delete_selected_item();

    assert!(matches!(
        state.status,
        Status::Error(ref message)
            if message == "Select a live or resurrectable session to delete it."
    ));
}

#[test]
fn delete_refuses_current_session() {
    let mut state = State::default();
    state.directories = vec![ZoxideDirectory {
        ranking: 1.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];
    state.session_manager.update_sessions(vec![SessionInfo {
        name: "repo".to_string(),
        is_current_session: true,
        ..SessionInfo::default()
    }]);

    state.delete_selected_item();

    assert!(matches!(
        state.status,
        Status::Error(ref message)
            if message == "Cannot delete the current session from inside itself."
    ));
}

#[test]
fn current_session_is_shown_but_not_selected() {
    let mut state = State::default();
    state.directories = vec![ZoxideDirectory {
        ranking: 2.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];
    state.session_manager.update_sessions(vec![
        SessionInfo {
            name: "repo".to_string(),
            is_current_session: true,
            ..SessionInfo::default()
        },
        SessionInfo {
            name: "other-live".to_string(),
            ..SessionInfo::default()
        },
    ]);

    let items = state.display_items();

    assert!(items.iter().any(|item| matches!(
        item,
        SessionItem::ExistingSession {
            name,
            is_current: true,
            ..
        } if name == "repo"
    )));

    state.clamp_selection();

    assert!(matches!(
        state.selected_item(),
        Some(SessionItem::ExistingSession {
            name,
            is_current: false,
            ..
        }) if name == "other-live"
    ));

    assert!(matches!(
        state.display_items().first(),
        Some(SessionItem::ExistingSession {
            name,
            is_current: true,
            ..
        }) if name == "repo"
    ));
}

#[test]
fn current_session_only_list_has_no_selectable_item() {
    let mut state = State::default();
    state.session_manager.update_sessions(vec![SessionInfo {
        name: "repo".to_string(),
        is_current_session: true,
        ..SessionInfo::default()
    }]);

    state.clamp_selection();

    assert!(state.selected_item().is_none());
    assert!(state.display_items().iter().any(|item| matches!(
        item,
        SessionItem::ExistingSession {
            name,
            is_current: true,
            ..
        } if name == "repo"
    )));
}

#[test]
fn shows_sessions_even_without_matching_directory() {
    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.session_manager.update_sessions(vec![SessionInfo {
        name: "loose-live".to_string(),
        is_current_session: false,
        ..SessionInfo::default()
    }]);
    state.session_manager.update_resurrectable_sessions(vec![(
        "loose-dead".to_string(),
        std::time::Duration::from_secs(1),
    )]);

    let items = state.display_items();

    assert!(items.iter().any(|item| matches!(
        item,
        SessionItem::ExistingSession { name, .. } if name == "loose-live"
    )));
    assert!(items.iter().any(|item| matches!(
        item,
        SessionItem::ResurrectableSession { name, .. } if name == "loose-dead"
    )));
}

#[test]
fn active_sessions_sort_before_other_items() {
    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.directories = vec![
        ZoxideDirectory {
            ranking: 2.0,
            directory: "/tmp/repo".to_string(),
            session_name: "repo".to_string(),
        },
        ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/other".to_string(),
            session_name: "other".to_string(),
        },
    ];
    state.session_manager.update_sessions(vec![
        SessionInfo {
            name: "loose-live".to_string(),
            ..SessionInfo::default()
        },
        SessionInfo {
            name: "repo".to_string(),
            ..SessionInfo::default()
        },
    ]);
    state.session_manager.update_resurrectable_sessions(vec![
        ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
        ("other".to_string(), std::time::Duration::from_secs(1)),
    ]);

    let items = state.display_items();
    let loose_live_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::ExistingSession { name, .. } if name == "loose-live"))
        .unwrap();
    let loose_dead_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "loose-dead"))
        .unwrap();
    let repo_index = items
        .iter()
        .position(
            |item| matches!(item, SessionItem::ExistingSession { name, .. } if name == "repo"),
        )
        .unwrap();
    let other_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "other"))
        .unwrap();

    assert!(repo_index < loose_live_index);
    assert!(loose_live_index < other_index);
    assert!(other_index < loose_dead_index);
}

#[test]
fn non_active_items_sort_by_zoxide_ranking() {
    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.directories = vec![
        ZoxideDirectory {
            ranking: 9.0,
            directory: "/tmp/high".to_string(),
            session_name: "high".to_string(),
        },
        ZoxideDirectory {
            ranking: 3.0,
            directory: "/tmp/mid".to_string(),
            session_name: "mid".to_string(),
        },
        ZoxideDirectory {
            ranking: 1.0,
            directory: "/tmp/low".to_string(),
            session_name: "low".to_string(),
        },
    ];
    state.session_manager.update_resurrectable_sessions(vec![
        ("mid".to_string(), std::time::Duration::from_secs(1)),
        ("loose".to_string(), std::time::Duration::from_secs(1)),
    ]);

    let items = state.display_items();
    let high_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::Directory { session_name, .. } if session_name == "high"))
        .unwrap();
    let mid_index = items
        .iter()
        .position(
            |item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "mid"),
        )
        .unwrap();
    let low_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::Directory { session_name, .. } if session_name == "low"))
        .unwrap();
    let loose_index = items
        .iter()
        .position(|item| matches!(item, SessionItem::ResurrectableSession { name, .. } if name == "loose"))
        .unwrap();

    assert!(high_index < mid_index);
    assert!(mid_index < low_index);
    assert!(low_index < loose_index);
}

#[test]
fn zoxide_only_filter_keeps_only_directory_backed_items() {
    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.directories = vec![ZoxideDirectory {
        ranking: 2.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];
    state.session_manager.update_sessions(vec![
        SessionInfo {
            name: "repo".to_string(),
            ..SessionInfo::default()
        },
        SessionInfo {
            name: "loose-live".to_string(),
            ..SessionInfo::default()
        },
    ]);
    state.session_manager.update_resurrectable_sessions(vec![
        ("repo.2".to_string(), std::time::Duration::from_secs(1)),
        ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
    ]);
    state.item_filter = ItemFilter::ZoxideOnly;

    let items = state.display_items();

    assert!(!items.is_empty());
    assert!(items.iter().all(SessionItem::is_zoxide_item));
}

#[test]
fn non_zoxide_filter_hides_directory_backed_items() {
    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.directories = vec![ZoxideDirectory {
        ranking: 2.0,
        directory: "/tmp/repo".to_string(),
        session_name: "repo".to_string(),
    }];
    state.session_manager.update_sessions(vec![
        SessionInfo {
            name: "repo".to_string(),
            ..SessionInfo::default()
        },
        SessionInfo {
            name: "loose-live".to_string(),
            ..SessionInfo::default()
        },
    ]);
    state.session_manager.update_resurrectable_sessions(vec![
        ("repo.2".to_string(), std::time::Duration::from_secs(1)),
        ("loose-dead".to_string(), std::time::Duration::from_secs(1)),
    ]);
    state.item_filter = ItemFilter::NonZoxideOnly;

    let items = state.display_items();

    assert!(!items.is_empty());
    assert!(items.iter().all(|item| !item.is_zoxide_item()));
}

#[test]
fn filters_out_treemin_managed_sessions() {
    let root = std::env::temp_dir().join(format!(
        "seshmin-registry-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&root);
    let registry = test_treemin_registry(&root);
    registry.add("repo-feature-a").unwrap();

    let mut state = State::default();
    state.config.show_resurrectable_sessions = true;
    state.session_manager.update_sessions(vec![
        SessionInfo {
            name: "repo-feature-a".to_string(),
            ..SessionInfo::default()
        },
        SessionInfo {
            name: "plain-session".to_string(),
            ..SessionInfo::default()
        },
    ]);
    state.session_manager.update_resurrectable_sessions(vec![
        (
            "repo-feature-a".to_string(),
            std::time::Duration::from_secs(1),
        ),
        ("plain-dead".to_string(), std::time::Duration::from_secs(1)),
    ]);

    let managed_sessions = registry.list().unwrap();
    state.filter_managed_sessions(&managed_sessions);

    let items = state.display_items();

    assert!(!items.iter().any(|item| matches!(
        item,
        SessionItem::ExistingSession { name, .. } if name == "repo-feature-a"
    )));
    assert!(!items.iter().any(|item| matches!(
        item,
        SessionItem::ResurrectableSession { name, .. } if name == "repo-feature-a"
    )));
    assert!(items.iter().any(|item| matches!(
        item,
        SessionItem::ExistingSession { name, .. } if name == "plain-session"
    )));
    assert!(items.iter().any(|item| matches!(
        item,
        SessionItem::ResurrectableSession { name, .. } if name == "plain-dead"
    )));
}

#[test]
fn directory_session_prefers_exact_resurrectable_match() {
    let mut state = State::default();
    state.session_manager.update_resurrectable_sessions(vec![
        ("repo".to_string(), std::time::Duration::from_secs(1)),
        ("repo.2".to_string(), std::time::Duration::from_secs(1)),
    ]);

    state.create_directory_session("/tmp/repo".to_string(), "repo".to_string(), true);

    assert!(matches!(state.status, Status::Loading));
}

#[test]
fn esc_clears_error_state() {
    let mut state = State::default();
    state.status = Status::Error("boom".to_string());
    state.sessions_loaded = true;
    state.directories_loaded = true;

    let handled = state.handle_key(KeyWithModifier {
        bare_key: BareKey::Esc,
        key_modifiers: BTreeSet::new(),
    });

    assert!(handled);
    assert!(matches!(state.status, Status::Ready));
}

#[test]
fn waits_for_sessions_and_directories_before_ready() {
    let mut state = State::default();

    state.directories_loaded = true;
    state.sync_status();
    assert!(matches!(state.status, Status::Busy(ref message) if message == "Loading sessions..."));

    state.directories_loaded = false;
    state.sessions_loaded = true;
    state.sync_status();
    assert!(
        matches!(state.status, Status::Busy(ref message) if message == "Loading zoxide directories...")
    );

    state.directories_loaded = true;
    state.sync_status();
    assert!(matches!(state.status, Status::Ready));
}
