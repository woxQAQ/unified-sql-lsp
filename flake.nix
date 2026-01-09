{
  description = "Tauri + Rust + Node (Nushell)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            nodejs_20
            clippy
            nushell
            playwright
            playwright-driver.browsers
            pkg-config
            openssl
          ];
          shellHook =
            #sh
            ''
              # Configure Playwright MCP for Nix environment
              # MCP needs to create configs in the browser directory (writable),
              # so we create a hybrid: writable user dir + symlinks to Nix browsers

              PLAYWRIGHT_USER_DIR="$HOME/.cache/ms-playwright"
              mkdir -p "$PLAYWRIGHT_USER_DIR"

              # Symlink Nix browsers into user directory if not already present
              NIX_BROWSERS="${pkgs.playwright-driver.browsers}"
              for browser in "$NIX_BROWSERS"/*; do
                if [ -d "$browser" ]; then
                  browser_name=$(basename "$browser")
                  if [ ! -e "$PLAYWRIGHT_USER_DIR/$browser_name" ]; then
                    ln -s "$browser" "$PLAYWRIGHT_USER_DIR/$browser_name"
                    echo "Linked $browser_name"
                  fi
                fi
              done

              # Set browsers path to writable user directory
              export PLAYWRIGHT_BROWSERS_PATH="$PLAYWRIGHT_USER_DIR"

              # Skip host requirements (Nix provides all dependencies)
              export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true

              echo "✓ Playwright MCP configured:"
              echo "  Browsers: $PLAYWRIGHT_BROWSERS_PATH"
            '';
          env = {
            # CARGO_TARGET_DIR = "/.cache/cargo-target";
            CARGO_INCREMENTAL = "1";
            RUST_BACKTRACE = "1";
          };
        };
      }
    );
}
