{
  description = "Rust library for email management.";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };
        in
        {
          # nix develop
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              # Nix LSP + formatter
              rnix-lsp
              nixpkgs-fmt

              # Rust env
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              cargo-watch
              rust-analyzer

              # Notmuch
              notmuch
            ];
          };
        }
      );
}
