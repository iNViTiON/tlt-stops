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

            # Rust/WASM worker build chain. worker-build is what
            # wrangler.toml's [build] command invokes; it drives wasm-pack,
            # wasm-bindgen, wasm-opt (binaryen) and esbuild for the shim.
            worker-build
            wasm-pack
            wasm-bindgen-cli
            binaryen
            esbuild

            # Frontend + tooling
            bun
            nodejs
            # wrangler comes from the project's package.json (node_modules/.bin)

            git
            curl
            jq
          ];

          # Top-level mkShell env attrs, NOT shellHook exports: only these
          # propagate through direnv's `use flake`, which captures the shell
          # via `nix print-dev-env`. shellHook-exported vars reach `nix
          # develop` but not reliably direnv.
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          # worker-build shells out to these; pin them to the nix-provided
          # binaries so nothing is fetched as an unpatched ELF at build time.
          WASM_BINDGEN_PATH = "${pkgs.wasm-bindgen-cli}/bin/wasm-bindgen";
          WASM_OPT_PATH = "${pkgs.binaryen}/bin/wasm-opt";
          ESBUILD_BINARY_PATH = "${pkgs.esbuild}/bin/esbuild";

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
