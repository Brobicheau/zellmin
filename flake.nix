{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1"; # unstable Nixpkgs
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:

    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.self.overlays.default
              ];
            };
          }
        );
    in
    {
      overlays.default = final: prev: {
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            (with stable; [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
            ])
            ++ [
              targets.wasm32-wasip1.stable.rust-std
            ]
          );
      };

      packages = forEachSupportedSystem (
        { pkgs, ... }:
        let
          zellminPlugins = pkgs.pkgsCross.wasi32.rustPlatform.buildRustPackage {
            pname = "zellmin-plugins";
            version = "unstable";
            src = self;

            cargoLock.lockFile = ./Cargo.lock;

            env.RUSTFLAGS = "-C linker=wasm-ld";
            nativeBuildInputs = [ pkgs.pkgsCross.wasi32.lld ];

            cargoBuildFlags = [
              "--target=wasm32-wasip1"
              "--workspace"
            ];
            doCheck = false;

            installPhase = ''
              runHook preInstall
              install -Dm644 target/wasm32-wasip1/release/treemin.wasm \
                $out/treemin.wasm
              install -Dm644 target/wasm32-wasip1/release/seshmin.wasm \
                $out/seshmin.wasm
              runHook postInstall
            '';
          };
        in
        {
          inherit zellminPlugins;

          treemin = pkgs.runCommand "treemin-wasm" { } ''
            install -Dm644 ${zellminPlugins}/treemin.wasm $out/treemin.wasm
          '';

          seshmin = pkgs.runCommand "seshmin-wasm" { } ''
            install -Dm644 ${zellminPlugins}/seshmin.wasm $out/seshmin.wasm
          '';

          default = zellminPlugins;
        }
      );

      devShells = forEachSupportedSystem (
        { pkgs, system }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustToolchain
              openssl
              curl
              pkg-config
              cargo-deny
              cargo-edit
              cargo-watch
              rust-analyzer
              self.formatter.${system}
            ];

            env = {
              # Required by rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );

      formatter = forEachSupportedSystem ({ pkgs, ... }: pkgs.nixfmt);
    };
}
