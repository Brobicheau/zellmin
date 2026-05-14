# Configuration Implementation Summary

## Overview

Added comprehensive configuration support to the treemin Zellij plugin with two configuration methods:
1. **Zellij KDL configuration** - Via plugin launch parameters
2. **Repository configuration** - Via `.treemin.toml` file in repo root

Configuration precedence: **Repo config > KDL config > Defaults**

## New Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `worktree_dir_name` | String | `.worktrees` | Worktree directory name (existing, now documented) |
| `session_prefix` | String? | None | Optional prefix for session names |
| `base_branch` | String? | None | Base branch for new branches |
| `remote` | String | `origin` | Git remote for fetching |
| `auto_fetch` | Boolean | `false` | Auto-fetch before creating worktree |
| `worktree_naming_pattern` | Enum | `branch` | Pattern: `branch`, `hash`, or `branch-hash` |

## Files Changed

### Core Implementation

**`Cargo.toml`**
- Added `serde` with derive features
- Added `toml` 0.8 for config parsing

**`src/config.rs`** (major rewrite)
- Added `WorktreeNamingPattern` enum
- Extended `Config` struct with all new fields
- Added `from_kdl()` method for Zellij configuration
- Added `from_toml()` method for repository configuration
- Added `merge()` method for config precedence
- Added comprehensive unit tests

**`src/naming.rs`**
- Updated `worktree_path()` to accept `&Config` and support naming patterns
- Updated `session_name()` to accept `&Config` and support session prefix

**`src/state.rs`**
- Added `kdl_config` field to store original KDL config
- Added `config_loaded` flag to track config loading state
- Added `ACTION_LOAD_REPO_CONFIG` constant
- Added `ACTION_FETCH_REMOTE` constant
- Updated `load()` to use `Config::from_kdl()`
- Added `load_repo_config()` method to read `.treemin.toml`
- Added `check_branch()` helper method
- Updated `begin_create_worktree()` to support auto-fetch
- Updated `create_worktree()` to use base_branch
- Updated `handle_run_command_result()` with new action handlers
- Updated `worktree_path()` and `session_name()` to pass config

**`src/ui.rs`**
- Updated `render()` to accept `&Config` instead of just `worktree_dir_name`
- Updated `render_ready()` to display all relevant config options
- Added `pattern_display()` helper for naming pattern visualization

### Documentation

**`README.md`**
- Updated with comprehensive configuration section
- Added configuration options table
- Added examples for both KDL and TOML config
- Added configuration examples for different workflows
- Added link to CONFIG.md

**`CONFIG.md`** (new)
- Comprehensive configuration guide
- Detailed explanation of each option
- Common workflow examples
- Troubleshooting section

**`.treemin.toml.example`** (new)
- Example repository configuration file
- Commented template for users

### Testing

**`src/config_tests.rs`** (new)
- Unit tests for default config
- Tests for KDL parsing
- Tests for TOML parsing
- Tests for config merging/precedence
- Tests for edge cases (empty strings, etc.)

## Behavior Changes

### Existing Functionality
- All existing behavior preserved
- `worktree_dir_name` works exactly as before
- Default behavior unchanged when no config provided

### New Functionality

1. **Repository Config Discovery**
   - On repo discovery, plugin attempts to read `.treemin.toml`
   - Merges repo config over KDL config
   - Shows "Loading repository configuration..." status

2. **Session Naming**
   - Optional prefix support: `{prefix}-repo-branch-hash`
   - Maintains sanitization and hash generation

3. **Worktree Naming Patterns**
   - `branch`: Uses sanitized branch name (preserves `/` structure)
   - `hash`: Uses 8-char hash of branch name
   - `branch-hash`: Combines both for uniqueness + readability

4. **Base Branch Support**
   - When creating new branches, uses specified base branch
   - Git command: `git worktree add -b <branch> <base_branch>`
   - Only affects new branch creation, not existing branches

5. **Auto-Fetch**
   - When enabled, runs `git fetch <remote>` before checking branch
   - Ensures latest remote branches are available
   - Shows "Fetching from remote..." status

## Usage Examples

### Minimal (defaults)
```kdl
LaunchOrFocusPlugin "file:/path/to/treemin.wasm" {
    floating true
}
```

### Customized via KDL
```kdl
LaunchOrFocusPlugin "file:/path/to/treemin.wasm" {
    floating true
    worktree_dir_name "trees"
    session_prefix "dev"
    auto_fetch "true"
}
```

### Repository Config
```toml
# .treemin.toml
worktree_dir_name = ".worktrees"
session_prefix = "proj"
base_branch = "develop"
remote = "origin"
auto_fetch = true
worktree_naming_pattern = "branch"
```

## Testing Recommendations

1. **Build Test**
   ```bash
   cargo build --release --target wasm32-wasip1
   ```

2. **Unit Tests**
   ```bash
   cargo test
   ```

3. **Integration Testing**
   - Test with no config (defaults)
   - Test with KDL config only
   - Test with repo config only
   - Test with both configs (precedence)
   - Test invalid TOML (error handling)
   - Test auto-fetch with/without network
   - Test each naming pattern
   - Test with base_branch set/unset

## Migration Notes

**For existing users:**
- No breaking changes
- Existing `worktree_dir_name` in KDL config continues to work
- New options are opt-in

**For upgrading:**
1. Update Cargo.toml dependencies
2. Rebuild plugin
3. Optionally add new config options
4. Optionally create `.treemin.toml` in repositories

## Future Enhancements

Potential additions (not implemented):
- `.treemin.toml.local` for personal overrides
- Per-branch config sections
- Config validation and schema
- Interactive config wizard
- Template support for branch names
- Multiple remote support
- Pre/post hooks for worktree creation
