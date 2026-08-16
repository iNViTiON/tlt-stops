{
  description = "TLT Stops - Tallinn public transport arrivals (Rust/WASM Cloudflare Worker + Svelte)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "tlt-stops";

          buildInputs = with pkgs; [
            # Rust toolchain. nixpkgs rustc ships wasm32-unknown-unknown std,
            # so no rust-overlay/fenix is needed for this crate.
            rustc
            cargo
            clippy
            rustfmt
            # wasm32 linking. The rustup toolchain ships a bundled rust-lld;
            # nixpkgs rustc does not, so `cargo build --target
            # wasm32-unknown-unknown` fails with "linker `lld` not found"
            # without this. `cargo check` doesn't link, so it hides the gap.
            lld

            # wrangler.toml's [build] command invokes this. It vendors its
            # own wasm-pack / wasm-bindgen / wasm-opt / esbuild, so those are
            # deliberately NOT in this list: putting them on PATH only risks
            # shadowing the versions worker-build and wrangler expect.
            worker-build

            # Frontend + tooling
            bun
            nodejs
            # wrangler comes from the project's package.json (node_modules/.bin)

            git
            curl
            jq
          ];

          # Deliberately no build-tool env vars here.
          #
          # CARGO_BUILD_TARGET: .cargo/config.toml already sets build.target
          # for this crate, and an env var outranks it — which would force
          # host tools built via `cargo install` to cross-compile to wasm32
          # too (openssl-sys then fails looking for a wasm OpenSSL sysroot).
          #
          # ESBUILD_BINARY_PATH: wrangler's esbuild JS wrapper refuses to
          # start against a binary of a different version ("Host version
          # 0.28.1 does not match binary version 0.27.2"). The vendored
          # esbuild is a static Go binary and runs fine on NixOS unaided.
          #
          # If anything here ever does need an env var, add it as a top-level
          # mkShell attr rather than a shellHook export — only top-level attrs
          # survive direnv's `use flake`, which captures via
          # `nix print-dev-env`.

          shellHook = ''
            # Project-local wrangler, version-matched to package.json
            export PATH="$PWD/node_modules/.bin:$PATH"

            echo "🚏 TLT Stops development environment"
            echo ""
            echo "  rustc  $(rustc --version | cut -d' ' -f2)   (target: wasm32-unknown-unknown)"
            echo "  cargo  $(cargo --version | cut -d' ' -f2)"
            echo "  worker-build $(worker-build --version)"
            echo "  bun    $(bun --version)"
            echo "  node   $(node --version)"
            echo ""
            echo "Backend:   cargo test --target x86_64-unknown-linux-gnu"
            echo "           cargo check      (defaults to wasm32 via .cargo/config.toml)"
            echo "           cargo clippy"
            echo "Frontend:  bun --cwd frontend run check | run build"
            echo "Deploy:    bunx wrangler deploy"
            echo ""
          '';
        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
