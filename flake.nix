{
  description = "TLT Stops — Rust/WASM Cloudflare Worker with a Svelte frontend";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          # rustc from nixpkgs already ships the wasm32-unknown-unknown std,
          # so the worker builds without an extra toolchain overlay.
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer

            # Linking the wasm32 target goes through lld, which nixpkgs ships
            # separately from rustc (unlike a rustup toolchain).
            lld

            # worker-build shells out to these when packaging the wasm module.
            binaryen
            wasm-bindgen-cli

            bun
            nodejs

            pkg-config
            openssl
          ];

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
    };
}
