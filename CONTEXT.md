# Project Context

This file captures the current working context for this repository so future work can resume without relying on chat history.

## Repo Overview

- This is a Rust workspace for Zellij plugins.
- Current workspace members:
  - `lib/host-storage`
  - `lib/session-registry`
  - `lib/ui`
  - `plugins/treemin`
  - `plugins/seshmin`
- Shared UI stays in `lib/ui`.
- Shared plugin persistence lives in `lib/host-storage`.
- Shared `treemin` session ownership metadata lives in `lib/session-registry`.

## Product Intent

- `treemin` manages git worktrees and related Zellij sessions.
- `seshmin` is an MVP-style session picker, similar in spirit to `zsm`, but using this repo's UI patterns.
- `seshmin` must hide `treemin`-managed sessions.

## Naming / Identity Changes Already Made

- Plugin packages and directories were renamed:
  - `treemin`
  - `seshmin`
- Old directories were renamed:
  - `plugins/zitree` -> `plugins/treemin`
  - `plugins/zessionz` -> `plugins/seshmin`
- Pane titles were updated to `treemin` and `seshmin`.
- Repo config file references were renamed from `.zitree.toml` to `.treemin.toml`.
- Docs updated for the new names in:
  - `README.md`
  - `AGENTS.md`
  - `CONFIG.md`
  - `QUICKREF.md`
  - `IMPLEMENTATION.md`

## Important Constraints / Decisions

- Keep shared UI in `lib/ui`.
- Do not add filepicker support.
- `seshmin` session names should use the selected directory name, not full paths.
- `default_layout` should be configurable; only ask for layout in-plugin when no default is set.
- `Ctrl+D` is the delete key in both plugins.
- `Esc` behavior:
  - `seshmin`: clear search / close / clear error
  - `treemin`: clear input if present, otherwise close
- `treemin` current session should be shown first but never selectable.
- `seshmin` current session should be shown first but never selectable.

## treemin Session Naming

- New `treemin` session names are based on branch name only.
- Branch collisions use numeric suffixes like `.2`, `.3`.
- New names should not include repo names, prefixes, or hashes.
- Legacy hashed and repo-based names are still preserved as matching candidates for old sessions only.
- Main worktree also keeps legacy repo-name matching for backward compatibility.
- Session name length is intentionally capped because of Zellij socket path constraints.

### Current naming API

- `plugins/treemin/src/naming.rs`
- `session_name(branch: &str, sibling_branches: &[String]) -> String`
- `session_name_candidates(repo_name, branch, config, sibling_branches, is_main_worktree) -> Vec<String>`

### Naming details

- Stable naming depends on passing the full relevant sibling branch set.
- Collision assignment sorts and deduplicates sibling branches before allocating names.
- If suffix allocation exhausts `.2` through `.1000`, naming falls back to a truncated `.overflow` suffix.
- Legacy candidate generation still includes hashed forms for matching old sessions.

## treemin Delete Behavior

- Deleting a linked worktree session should also delete the linked git worktree.
- If deleting a `treemin` worktree entry with no live session, delete the worktree directly.
- Deleting the main repository worktree is refused.
- There was active churn around stale async callbacks after delete; current intent is:
  - after delete, return to `Status::Ready`
  - ignore stale `FetchRemote`, `CheckBranch`, `CreateWorktree`, and `CreateSession` results while `pending_delete` is set

## treemin Session Ownership Registry

- `treemin`-managed session metadata is stored under `/tmp` so other plugins can inspect it.
- Registry file path:
  - `/tmp/treemin/sessions/managed.txt`
- Format:
  - plain text
  - one session name per line
- `treemin` adds names on successful session creation or reuse of matching live sessions.
- `treemin` removes names on successful delete.

## seshmin Behavior Completed So Far

- Shows all non-`treemin` active sessions.
- Shows all non-`treemin` resurrectable sessions.
- Filters out `treemin`-managed sessions by reading host storage metadata, not by name heuristics.
- Waits for both session data and zoxide directories before becoming ready.
- Does not show the list until both are loaded.
- Ordering rules implemented:
  - current session first
  - current session shown but not selectable
  - other live sessions before non-live
  - among live sessions, directory-backed before loose sessions
  - remaining items ordered by zoxide rank
- Search and selection behavior updated so the current session stays non-selectable.
- `Ctrl+F` cycles item filters:
  - `all`
  - `zoxide only`
  - `non-zoxide only`
- Conservative session-name max is enforced in `seshmin` as well.

## treemin Behavior Completed So Far

- `Esc` clears branch input if present; otherwise closes plugin.
- Current session/worktree row is sorted first and is not selectable.
- Selection is `Option<usize>` so a lone current row has no cursor selection.
- Linked worktree delete removes both session and worktree.
- Worktree entries with no live session can be deleted directly.
- Main repo worktree delete is refused.

## Recent treemin State Integration Work

The latest work updated `plugins/treemin/src/state.rs` to use the new branch-only naming model.

### Updated in state.rs

- New/generated displayed session names now come from `naming::session_name(branch, sibling_branches)`.
- Matching of live sessions now passes sibling branch context into `naming::session_name_candidates(...)`.
- `state.rs` now uses a small helper to build sibling branch lists from known worktrees.
- Main worktree rows now display `main` as the generated session name, while still matching legacy repo-name sessions like `repo`.
- Linked worktree rows now display branch-only names like `feature-test`, while still matching legacy hashed sessions like `repo-feature-test-727724f6`.

### Important current behavior

- Display/generated session name and matched live session name may differ during migration.
- Example:
  - displayed/generated session name: `feature-test`
  - matched existing live session name: `repo-feature-test-727724f6`

## Files Most Relevant To Current Work

- `Cargo.toml`: workspace members
- `lib/host-storage/src/lib.rs`: `/tmp`-rooted storage helper
- `lib/session-registry/src/lib.rs`: `treemin` managed session registry
- `plugins/treemin/src/naming.rs`: branch-only naming, collision handling, legacy matching candidates
- `plugins/treemin/src/state.rs`: current `treemin` state, matching, delete flow, selection logic, tests
- `plugins/treemin/src/storage.rs`: registry helper
- `plugins/treemin/src/config.rs`: config parsing and `.treemin.toml`
- `plugins/treemin/src/commands/mod.rs`: repo config loading
- `plugins/treemin/src/ui/mod.rs`: rendering and optional selection behavior
- `plugins/seshmin/src/state.rs`: main `seshmin` behavior, filtering, ordering, loading gates
- `plugins/seshmin/src/session/manager.rs`: session retention helpers
- `plugins/seshmin/src/session/types.rs`: picker item types and ordering/selectability logic
- `plugins/seshmin/src/ui/mod.rs`: rendering and filter display
- `plugins/seshmin/src/zoxide/mod.rs`: zoxide parsing and session name generation
- `plugins/seshmin/src/zoxide/search.rs`: search ordering and current-session handling

## CI Pipeline

- GitHub Actions workflow lives at `.github/workflows/ci.yml`.
- It runs on pushes and on pull requests.
- The workflow installs stable Rust with `wasm32-wasip1`, `rustfmt`, `clippy`, and `just`.
- The workflow entry point is `just ci`.
- `just ci` runs:
  - `just check`
  - `just test`
  - `just build`
- `just fmt-check` and `just clippy` are available separately for stricter cleanup verification.

## Known Environment Limitation

- Local Rust verification is currently blocked in this environment because `cargo` is unavailable.
- Failed command:
  - `/usr/bin/bash: line 1: cargo: command not found`
- Because of that, local `cargo` / `just` validation has not been run from this session.

## Verification To Run When Toolchain Is Available

- `just fmt`
- `just clippy`
- `just test`
- `just build`

Per repo guidance:

- use `just` commands instead of guessing cargo flags
- wasm target defaults to `wasm32-wasip1`
- `just test` runs host-target unit tests, not wasm tests

## Most Likely Next Steps

- Run formatting/tests/build once the Rust toolchain is available.
- Fix any compile/test regressions from the recent `treemin` naming API transition.
- Recheck `treemin` state behavior around delete-flow callbacks and rebuild timing if tests expose issues.

## Handoff Summary

If resuming work later, start by reading:

- `CONTEXT.md`
- `AGENTS.md`
- `plugins/treemin/src/naming.rs`
- `plugins/treemin/src/state.rs`

That is the shortest path back to the current work.
