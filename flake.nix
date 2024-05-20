{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      overlays = [ (import rust-overlay) ];
      forAllSystems =
        function:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (
          system: function (import nixpkgs { inherit system overlays; })
        );
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = md-tui;
        md-tui =
          (pkgs.makeRustPlatform {
            cargo = pkgs.rust-bin.stable.latest.minimal;
            rustc = pkgs.rust-bin.stable.latest.minimal;
          }).buildRustPackage
            {
              inherit (cargoToml.package) name version;
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              meta.mainProgram = "mdt";
            };
      });
      devShells = forAllSystems (
        pkgs:
        let
          toolchain = pkgs.rust-bin.stable.latest.default.override { extensions = [ "rust-src" ]; };
        in
        {
          default = pkgs.mkShell {
            buildInputs = [ toolchain ];

            RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
            RUST_BACKTRACE = 1;
          };
        }
      );
    };
}
