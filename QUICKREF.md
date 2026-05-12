# Zitree Configuration Quick Reference

## Configuration Methods

```kdl
# Method 1: Zellij KDL Config
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" {
    floating true
    worktree_dir_name ".worktrees"
    session_prefix "wt"
    base_branch "main"
    remote "origin"
    auto_fetch "true"
    worktree_naming_pattern "branch"
}
```

```toml
# Method 2: Repository .zitree.toml
worktree_dir_name = ".worktrees"
session_prefix = "wt"
base_branch = "main"
remote = "origin"
auto_fetch = true
worktree_naming_pattern = "branch"
```

**Precedence:** Repo > KDL > Defaults

## Options

| Option | Values | Default |
|--------|--------|---------|
| `worktree_dir_name` | string | `.worktrees` |
| `session_prefix` | string? | _none_ |
| `base_branch` | string? | _none_ |
| `remote` | string | `origin` |
| `auto_fetch` | bool | `false` |
| `worktree_naming_pattern` | `branch` \| `hash` \| `branch-hash` | `branch` |

## Naming Patterns

```
branch:       .worktrees/feature/xyz
hash:         .worktrees/a1b2c3d4
branch-hash:  .worktrees/feature-xyz-a1b2c3d4
```

## Session Names

```
No prefix:     myrepo-feature-abc123
With prefix:   wt-myrepo-feature-abc123
```

## Common Configs

**Minimal:**
```kdl
LaunchOrFocusPlugin "file:/path/to/zitree.wasm" { floating true }
```

**Team workflow:**
```toml
base_branch = "develop"
auto_fetch = true
```

**Fork workflow:**
```toml
remote = "upstream"
auto_fetch = true
base_branch = "main"
```

**Compact names:**
```toml
worktree_naming_pattern = "hash"
```

## Files

- `.zitree.toml` - Repository config (commit this)
- `.zitree.toml.example` - Template
- `CONFIG.md` - Full documentation

## See Also

- [README.md](README.md) - Installation & usage
- [CONFIG.md](CONFIG.md) - Detailed guide
- [IMPLEMENTATION.md](IMPLEMENTATION.md) - Technical details
