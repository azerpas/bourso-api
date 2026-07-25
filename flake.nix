{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix/3cfd774b0a530725a077e17354fbdb87ea1c4aad";
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-parts, pre-commit-hooks, v_flakes }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;

      perSystem = { config, self', inputs', system, ... }:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          rust = pkgs.rust-bin.stable."1.93.0".default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          };
          manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
          pname = manifest.name;
          #rs = v_flakes.rs { inherit pkgs rust; };
          github = v_flakes.github {
            enable = true;
            inherit pkgs pname;
            syncFork = true;
            gitignore.extend = ''
              						*.chls
              						*.chlz
              						'';
          };
          combined = v_flakes.utils.combine [ github ];
        in
        {
          packages.default = (pkgs.makeRustPlatform { rustc = rust; cargo = rust; inherit stdenv; }).buildRustPackage {
            inherit pname;
            version = manifest.version;

            cargoLock.lockFile = ./Cargo.lock;
            src = pkgs.lib.cleanSource ./.;

            buildInputs = [ pkgs.openssl ];
            nativeBuildInputs = [ pkgs.pkg-config ];
          };

          devShells.default = with pkgs; mkShell {
            inherit stdenv;
            shellHook =
              pre-commit-check.shellHook +
              combined.shellHook +
              ''
                cp -f ${(v_flakes.files.treefmt) {inherit pkgs;}} ./.treefmt.toml
              '';
            env = {
              RUST_BACKTRACE = 1;
              RUST_LIB_BACKTRACE = 0;
            };

            packages = [
              mold
              openssl
              pkg-config
              rust
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;
          };
        };
    };
}
