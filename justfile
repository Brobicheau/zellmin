wasm_target := env_var_or_default("WASM_TARGET", "wasm32-wasip1")

treemin_plugin_name := "treemin"
seshmin_plugin_name := "seshmin"

treemin_debug_plugin_path := "target/" + wasm_target + "/debug/" + treemin_plugin_name + ".wasm"
treemin_release_plugin_path := "target/" + wasm_target + "/release/" + treemin_plugin_name + ".wasm"
seshmin_debug_plugin_path := "target/" + wasm_target + "/debug/" + seshmin_plugin_name + ".wasm"
seshmin_release_plugin_path := "target/" + wasm_target + "/release/" + seshmin_plugin_name + ".wasm"

# List available tasks
[group('Help')]
default:
    @just --list


# Check all crates for the configured wasm target
[group('Build')]
check:
    cargo check --target {{wasm_target}} --workspace

# Build all crates for the configured wasm target
[group('Build')]
build:
    cargo build --target {{wasm_target}} --workspace

# Build optimized plugin artifacts for the configured wasm target
[group('Build')]
release:
    cargo build --release --target {{wasm_target}} --workspace

# Format Rust source files
[group('Quality')]
fmt:
    cargo fmt

# Run clippy for the configured wasm target and deny warnings
[group('Quality')]
clippy:
    cargo clippy --target {{wasm_target}} --workspace -- -D warnings

# Run host-target Rust tests
[group('Quality')]
test:
    LD_LIBRARY_PATH="$(nix build --no-link --print-out-paths nixpkgs#curl.out)/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" cargo test --workspace

[group('Quality')]
debug-logs:
    tail -f "$(dirname "$(mktemp --dry)")/zellij-$(id -u)/zellij-log/zellij.log" | grep -E "DEBUG"

[group('Quality')]
plugin-logs:
    tail -f "$(dirname "$(mktemp --dry)")/zellij-$(id -u)/zellij-log/zellij.log"

# Print the debug treemin plugin wasm path
[group('Treemin')]
treemin-plugin-path:
    @printf "%s\n" "{{treemin_debug_plugin_path}}"

# Print the release treemin plugin wasm path
[group('Treemin')]
treemin-release-plugin-path:
    @printf "%s\n" "{{treemin_release_plugin_path}}"

# Open or focus the debug treemin plugin in Zellij
[group('Treemin')]
treemin-open:
    zellij action launch-or-focus-plugin "file:$PWD/{{treemin_debug_plugin_path}}" --floating --skip-plugin-cache

# Open or focus the release treemin plugin in Zellij
[group('Treemin')]
treemin-open-release:
    zellij action launch-or-focus-plugin "file:$PWD/{{treemin_release_plugin_path}}" --floating --skip-plugin-cache

# Print the debug seshmin plugin wasm path
[group('Seshmin')]
seshmin-plugin-path:
    @printf "%s\n" "{{seshmin_debug_plugin_path}}"

# Print the release seshmin plugin wasm path
[group('Seshmin')]
seshmin-release-plugin-path:
    @printf "%s\n" "{{seshmin_release_plugin_path}}"

# Open or focus the debug seshmin plugin in Zellij
[group('Seshmin')]
seshmin-open:
    zellij action launch-or-focus-plugin "file:$PWD/{{seshmin_debug_plugin_path}}" --floating --skip-plugin-cache

# Open or focus the release seshmin plugin in Zellij
[group('Seshmin')]
seshmin-open-release:
    zellij action launch-or-focus-plugin "file:$PWD/{{seshmin_release_plugin_path}}" --floating --skip-plugin-cache

[group('Seshmin')]
seshmin-kill-all:
    #!/usr/bin/env nu
    zellij action list-panes | from ssv | select PANE_ID TITLE | where TITLE == "seshmin" | get PANE_ID | each { |id| zellij action close-pane -p $id }

[group('Seshmin')]
seshmin-reload: build seshmin-kill-all seshmin-open

