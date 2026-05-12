# Zitree Configuration Guide

This guide explains how to configure the zitree Zellij plugin for your workflow.

## Configuration Methods

Zitree supports two configuration methods that work together:

1. **Zellij KDL Configuration** - Global or keybinding-specific settings
2. **Repository Configuration** - Per-repository `.zitree.toml` file

**Precedence:** Repository config > KDL config > Defaults

## Quick Start

### Minimal Setup

```kdl
bind "Alt w" {
    LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
        floating true
    }
}
```

This uses all defaults:
- Worktrees in `.worktrees/`
- Session names: `repo-branch-hash`
- No auto-fetch
- Branch-based naming

### Recommended Setup

```kdl
bind "Alt w" {
    LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
        floating true
        move_to_focused_tab true
        worktree_dir_name ".worktrees"
        auto_fetch "true"
    }
}
```

## Configuration Options

### `worktree_dir_name`

**Type:** String  
**Default:** `.worktrees`  
**Description:** Directory name for worktrees relative to repository root.

**Examples:**
```kdl
worktree_dir_name ".worktrees"   # Default
worktree_dir_name "trees"        # Custom name
worktree_dir_name ".git/trees"   # Inside .git directory
```

**TOML:**
```toml
worktree_dir_name = ".worktrees"
```

**Result:** Worktrees created at `<repo>/.worktrees/<branch>`

---

### `session_prefix`

**Type:** String (optional)  
**Default:** None  
**Description:** Prefix for Zellij session names.

**Examples:**
```kdl
session_prefix "wt"      # Sessions: wt-repo-branch-hash
session_prefix "dev"     # Sessions: dev-repo-branch-hash
```

**TOML:**
```toml
session_prefix = "wt"
```

**Use cases:**
- Distinguish work vs personal projects
- Namespace by team or project type
- Organize session list

---

### `base_branch`

**Type:** String (optional)  
**Default:** None  
**Description:** Base branch for creating new branches.

**Examples:**
```kdl
base_branch "main"      # New branches from main
base_branch "develop"   # New branches from develop
```

**TOML:**
```toml
base_branch = "main"
```

**Behavior:**
- When creating a **new** branch, it branches from `base_branch`
- For **existing** branches, this has no effect
- Without this, git uses your current HEAD

---

### `remote`

**Type:** String  
**Default:** `origin`  
**Description:** Git remote to use for fetching.

**Examples:**
```kdl
remote "upstream"   # Fork workflow
remote "origin"     # Standard workflow
```

**TOML:**
```toml
remote = "upstream"
```

**Use cases:**
- Fork workflows (fetch from upstream)
- Multiple remotes
- Custom remote names

---

### `auto_fetch`

**Type:** Boolean  
**Default:** `false`  
**Description:** Automatically fetch from remote before creating worktree.

**Examples:**
```kdl
auto_fetch "true"    # Always fetch first
auto_fetch "false"   # Manual fetching
```

**TOML:**
```toml
auto_fetch = true
```

**Behavior:**
- `true`: Runs `git fetch <remote>` before checking branch
- Ensures latest remote branches are available
- Adds slight delay for network operation

---

### `worktree_naming_pattern`

**Type:** Enum (`branch` | `hash` | `branch-hash`)  
**Default:** `branch`  
**Description:** Pattern for worktree directory names.

**Options:**

| Pattern | Example | Use Case |
|---------|---------|----------|
| `branch` | `feature/xyz` → `.worktrees/feature/xyz` | Readable, organized by slashes |
| `hash` | `feature/xyz` → `.worktrees/a1b2c3d4` | Short, no special chars |
| `branch-hash` | `feature/xyz` → `.worktrees/feature-xyz-a1b2c3d4` | Readable + unique |

**Examples:**
```kdl
worktree_naming_pattern "branch"       # Default
worktree_naming_pattern "hash"         # Compact
worktree_naming_pattern "branch-hash"  # Hybrid
```

**TOML:**
```toml
worktree_naming_pattern = "branch"
```

## Common Workflows

### Solo Developer

```kdl
# Zellij config - simple and fast
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    floating true
    worktree_dir_name "trees"
    session_prefix "work"
}
```

### Team with Shared Config

**Zellij config:**
```kdl
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    floating true
}
```

**Repository `.zitree.toml`:**
```toml
# Committed to repo, shared by team
base_branch = "develop"
auto_fetch = true
remote = "origin"
worktree_naming_pattern = "branch"
```

### Fork Workflow

```toml
# .zitree.toml for upstream tracking
remote = "upstream"
auto_fetch = true
base_branch = "main"
```

### Compact Worktree Names

```toml
# Good for long branch names or special characters
worktree_naming_pattern = "hash"
```

### Organized by Feature

```toml
# Preserves branch hierarchy
worktree_naming_pattern = "branch"
# Results in: .worktrees/feature/auth, .worktrees/bugfix/login
```

## Repository Configuration

Create `.zitree.toml` in your repository root:

```toml
# Example complete configuration
worktree_dir_name = ".worktrees"
session_prefix = "proj"
base_branch = "develop"
remote = "origin"
auto_fetch = true
worktree_naming_pattern = "branch-hash"
```

**Advantages:**
- Version controlled
- Shared across team
- Per-repository customization
- Overrides personal Zellij config

**Gitignore:**
You may want to ignore `.zitree.toml.local` for personal overrides (not currently supported, but could be added).

## Tips

1. **Start simple** - Use defaults first, add config as needed
2. **Team config in repo** - Commit `.zitree.toml` for consistency
3. **Auto-fetch for teams** - Ensures everyone has latest branches
4. **Session prefix for organization** - Group sessions by project type
5. **Hash pattern for compatibility** - Avoid filesystem issues with special chars

## Troubleshooting

**Config not loading:**
- Check `.zitree.toml` is in repository root (where `.git` is)
- Verify TOML syntax with `cat .zitree.toml`
- Plugin shows "Loading repository configuration..." status

**Wrong config applied:**
- Remember: Repo config > KDL config > Defaults
- Check both `.zitree.toml` and Zellij keybinding config
- Reload plugin to pick up config changes

**Auto-fetch slow:**
- Disable `auto_fetch` if network is slow
- Manual fetch: `git fetch` before using plugin

## See Also

- [README.md](README.md) - Plugin overview and installation
- [Zellij Documentation](https://zellij.dev) - Zellij configuration
- [Git Worktree Documentation](https://git-scm.com/docs/git-worktree) - Understanding worktrees
