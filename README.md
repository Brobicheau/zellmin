# zitree

Zellij plugin for creating Git worktrees and switching into a session rooted at the new worktree.

## Behavior

- Uses `<repo>/.worktrees/<branch>` as the worktree path.
- Uses a sanitized `<repo>-<branch>-<hash>` Zellij session name.
- Reuses an existing session only after `git worktree add` succeeds for the expected path.
- Fails clearly if `git worktree add` fails, including when the branch is already checked out in another worktree.

## Build

This repository is a Rust Zellij plugin crate. Build it for WASI:

```bash
cargo build --release --target wasm32-wasip1
```

If your toolchain still expects the legacy target name, use `wasm32-wasi` instead.

Expected plugin artifact:

```text
target/wasm32-wasi/release/zitree.wasm
target/wasm32-wasip1/release/zitree.wasm
```

## Load In Zellij

Example keybinding:

```kdl
keybinds {
    normal {
        bind "Alt w" {
            LaunchOrFocusPlugin "file:/absolute/path/to/zitree/target/wasm32-wasip1/release/zitree.wasm" {
                floating true
                move_to_focused_tab true
                worktree_dir_name ".worktrees"
            }
        }
    }
}
```

You can also launch it directly:

```bash
zellij action launch-plugin "file:/absolute/path/to/zitree/target/wasm32-wasip1/release/zitree.wasm" --floating
```

## Usage

1. Focus the plugin pane.
2. Type the branch name.
3. Press `Enter`.

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
