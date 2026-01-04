{
  description = "Unified SQL LSP development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Build dependencies
            pkg-config
            openssl

            # System libraries (fixes -liconv linker error)
            libiconv

            # Tree-sitter CLI (for grammar development)
            nodePackages.tree-sitter-cli

            # Node.js (required by tree-sitter)
            nodejs

            # Other tools
            git
          ];

          # Environment variables
          shellHook = ''
            # Ensure Rust can find OpenSSL
            export OPENSSL_DIR=${pkgs.openssl.dev}
            export OPENSSL_LIB=${pkgs.openssl.out}/lib
            export OPENSSL_INCLUDE_DIR=${pkgs.openssl.dev}/include

            # Ensure Rust can find libiconv
            export LIBICONV_LIB=${pkgs.libiconv}/lib
            export LIBICONV_INCLUDE=${pkgs.libiconv}/include

            # Add pkg-config library path
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            echo "✅ Unified SQL LSP development environment loaded"
            echo "Rust version: $(rustc --version)"
            echo "Tree-sitter version: $(tree-sitter --version)"
          '';
        };
      }
    );
}
