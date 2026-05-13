# zitree

Zellij plugin workspace.

Crates:
- `lib/ui`: shared boxed terminal UI primitives used by both plugins
- `plugins/zitree`: create Git worktrees and switch into a session rooted at the new worktree
- `plugins/zessionz`: zoxide-powered session picker using this repo's boxed terminal UI components

## Workspace Build

Build the original plugin:

```bash
cargo build --release --target wasm32-wasip1 -p zitree
```

Build the new plugin:

```bash
cargo build --release --target wasm32-wasip1 -p zessionz
```

Expected plugin artifacts:

```text
target/wasm32-wasip1/release/zitree.wasm
target/wasm32-wasip1/release/zessionz.wasm
```

## zessionz

`zessionz` is the MVP sibling plugin modeled after `zsm`.

It now consumes the shared `lib/ui` crate instead of maintaining its own copy of the box-panel and ANSI style helpers.

Behavior:
- queries `zoxide query -l -s` and ranks directories by score
- merges live Zellij sessions with the generated directory list
- supports fuzzy search across sessions and directories
- `Enter` switches an existing session and creates a directory session immediately when `default_layout` is configured
- `Enter` opens the in-plugin layout picker only when `default_layout` is not configured
- `Ctrl+Enter` quick-creates the selected directory using the configured default layout when available
- skips filepicker support for now

Example keybinding:

```kdl
keybinds {
    normal {
        bind "Alt z" {
            LaunchOrFocusPlugin "file:/absolute/path/to/zitree/target/wasm32-wasip1/release/zessionz.wasm" {
                floating true
                move_to_focused_tab true
                default_layout "development"
                session_separator "."
                show_resurrectable_sessions "false"
                search_directories "/home/user/projects|/home/user/src"
                base_paths "/home/user/projects|/home/user/src"
                ignored_directories "/home/user/projects/archive|/home/user/projects/tmp"
            }
        }
    }
}
```

Configuration options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_layout` | String | _(none)_ | Layout used for session creation; when set, `Enter` skips the in-plugin layout picker |
| `session_separator` | String | `.` | Separator used when generating session names |
| `show_resurrectable_sessions` | Boolean | `false` | Include resurrectable sessions in the list |
| `search_directories` | String | _(none)_ | Pipe-separated directories to include from zoxide results, including descendants |
| `base_paths` | String | _(none)_ | Pipe-separated path prefixes stripped before name generation |
| `ignored_directories` | String | _(none)_ | Pipe-separated directories excluded from zoxide results, including descendants |

## zitree

Zellij plugin for creating Git worktrees and switching into a session rooted at the new worktree.

`zitree` now lives in `plugins/zitree` and also consumes the shared `lib/ui` crate.

## Behavior

- Uses `<repo>/.worktrees/<branch>` as the worktree path.
- Uses a sanitized `<repo>-<branch>-<hash>` Zellij session name.
- Automatically shortens overlong session names to avoid Zellij IPC socket path limits, especially on macOS where `$TMPDIR` is often long.
- Creates or switches to a deterministic session only after `git worktree add` succeeds for the expected path.
- Shows only Git worktrees from the current repository in the session list.
- Creates the selected worktree session on demand when you press `Enter` and that worktree does not already have a live session.
- Fails clearly if `git worktree add` fails, including when the branch is already checked out in another worktree.

## Build

This repository is a Rust workspace. Build `zitree` for WASI:

```bash
cargo build --release --target wasm32-wasip1 -p zitree
```

If your toolchain still expects the legacy target name, use `wasm32-wasi` instead.

The crate is pinned to `zellij-tile 0.44.1` to match Zellij `0.44.1`.

Expected plugin artifact:

```text
target/wasm32-wasi/release/zitree.wasm
target/wasm32-wasip1/release/zitree.wasm
```

## Load In Zellij

Example keybinding with configuration:

```kdl
keybinds {
    normal {
        bind "Alt w" {
            LaunchOrFocusPlugin "file:/absolute/path/to/zitree/target/wasm32-wasip1/release/zitree.wasm" {
                floating true
                move_to_focused_tab true
                worktree_dir_name ".worktrees"
                session_prefix "wt"
                base_branch "main"
                remote "origin"
                auto_fetch "true"
                worktree_naming_pattern "branch"
            }
        }
    }
}
```

You can also launch it directly:

```bash
zellij action launch-plugin "file:/absolute/path/to/zitree/target/wasm32-wasip1/release/zitree.wasm" --floating
```

## Configuration

The plugin supports configuration through two methods:
1. **Zellij KDL configuration** (in your keybindings)
2. **Repository config file** (`.zitree.toml` in repo root)

Repository config takes precedence over KDL config, which takes precedence over defaults.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `worktree_dir_name` | String | `.worktrees` | Directory name for worktrees relative to repo root |
| `session_prefix` | String | _(none)_ | Optional prefix for Zellij session names (e.g., "wt" → "wt-repo-branch-hash"); overlong names are shortened automatically |
| `base_branch` | String | _(none)_ | Base branch to track from when creating new branches (e.g., "main", "develop") |
| `remote` | String | `origin` | Git remote to use when checking out branches |
| `auto_fetch` | Boolean | `false` | Whether to fetch from remote before creating worktree |
| `worktree_naming_pattern` | String | `branch` | Pattern for worktree directory naming: "branch", "hash", or "branch-hash" |

### Zellij KDL Configuration

Add configuration options to your Zellij keybinding:

```kdl
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    floating true
    move_to_focused_tab true
    worktree_dir_name ".worktrees"
    session_prefix "wt"
    base_branch "main"
    auto_fetch "true"
}
```

### Repository Configuration

Create a `.zitree.toml` file in your repository root:

```toml
# Directory for worktrees
worktree_dir_name = ".worktrees"

# Optional session prefix
session_prefix = "wt"

# Base branch for new branches
base_branch = "main"

# Git remote to use
remote = "origin"

# Auto-fetch before creating worktree
auto_fetch = true

# Worktree naming pattern: "branch", "hash", or "branch-hash"
worktree_naming_pattern = "branch"
```

**Benefits of repository config:**
- Share configuration across team members
- Commit to version control
- Override Zellij config per-repository
- Supports more complex configuration in TOML format

### Configuration Examples

**Minimal setup (defaults):**
```kdl
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    floating true
}
```

**Team workflow with auto-fetch:**
```toml
# .zitree.toml
base_branch = "develop"
auto_fetch = true
remote = "origin"
session_prefix = "dev"
```

**Personal workflow with custom naming:**
```kdl
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    worktree_dir_name "trees"
    worktree_naming_pattern "branch-hash"
    session_prefix "work"
}
```

## Usage

1. Focus the plugin pane.
2. Type the branch name.
3. Press `Enter`.

With an empty branch input, you can also:

1. Use `Up` / `Down` to select a repository worktree.
2. Press `Enter` to switch to its session, creating the session if needed.
3. Press `Delete` to delete the selected non-current session.

The plugin will:

1. Discover the repo root.
2. Check whether the branch already exists locally.
3. Create `<repo>/.worktrees/<branch>` using `git worktree add`.
4. Switch to or create a session named from the repo, branch, and a short branch hash.
5. Use the worktree as that session's cwd.

## Notes

- The plugin requests `ReadApplicationState`, `ChangeApplicationState`, and `RunCommands` permissions.
- First version keeps the UI intentionally small: a single branch input flow.
- Existing-branch handling is delegated to `git worktree add`; the plugin surfaces the failure message directly.
- See [CONFIG.md](CONFIG.md) for detailed configuration guide and examples.
