wasm_target := env_var_or_default("WASM_TARGET", "wasm32-wasip1")
plugin_name := "zitree"
plugin_path := `printf "target/%s/debug/%s.wasm" "{{wasm_target}}" "{{plugin_name}}"`
release_plugin_path := `printf "target/%s/release/%s.wasm" "{{wasm_target}}" "{{plugin_name}}"`

default:
    @just --list

install-wasm-target:
    rustup target add {{wasm_target}}

install-wasm-target-legacy:
    rustup target add wasm32-wasi

check:
    cargo check --target {{wasm_target}}

build:
    cargo build --target {{wasm_target}}

release:
    cargo build --release --target {{wasm_target}}

fmt:
    cargo fmt

clippy:
    cargo clippy --target {{wasm_target}} -- -D warnings

test:
    cargo test

plugin-path:
    @printf "%s\n" "{{plugin_path}}"

release-plugin-path:
    @printf "%s\n" "{{release_plugin_path}}"

reload:
    zellij action start-or-reload-plugin "file:$PWD/{{plugin_path}}"

reload-release:
    zellij action start-or-reload-plugin "file:$PWD/{{release_plugin_path}}"
