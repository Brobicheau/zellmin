wasm_target := env_var_or_default("WASM_TARGET", "wasm32-wasip1")
plugin_name := "zitree"
plugin_path := `printf "target/%s/debug/%s.wasm" "{{wasm_target}}" "{{plugin_name}}"`
release_plugin_path := `printf "target/%s/release/%s.wasm" "{{wasm_target}}" "{{plugin_name}}"`
zessionz_plugin_path := `printf "target/%s/debug/zessionz.wasm" "{{wasm_target}}"`
zessionz_release_plugin_path := `printf "target/%s/release/zessionz.wasm" "{{wasm_target}}"`

default:
    @just --list

install-wasm-target:
    rustup target add {{wasm_target}}

install-wasm-target-legacy:
    rustup target add wasm32-wasi

check:
    cargo check --target {{wasm_target}} --workspace

build:
    cargo build --target {{wasm_target}} --workspace

release:
    cargo build --release --target {{wasm_target}} --workspace

fmt:
    cargo fmt

clippy:
    cargo clippy --target {{wasm_target}} --workspace -- -D warnings

test:
    cargo test --workspace

plugin-path:
    @printf "%s\n" "{{plugin_path}}"

release-plugin-path:
    @printf "%s\n" "{{release_plugin_path}}"

reload:
    zellij action launch-or-focus-plugin "file:$PWD/target/wasm32-wasip1/debug/zitree.wasm" --floating --skip-plugin-cache

reload-release:
    zellij action launch-or-focus-plugin "file:$PWD/{{release_plugin_path}}" --floating --skip-plugin-cache

zessionz-plugin-path:
    @printf "%s\n" "{{zessionz_plugin_path}}"

zessionz-release-plugin-path:
    @printf "%s\n" "{{zessionz_release_plugin_path}}"

reload-zessionz:
    zellij action launch-or-focus-plugin "file:$PWD/{{zessionz_plugin_path}}" --floating --skip-plugin-cache

reload-zessionz-release:
    zellij action launch-or-focus-plugin "file:$PWD/{{zessionz_release_plugin_path}}" --floating --skip-plugin-cache
