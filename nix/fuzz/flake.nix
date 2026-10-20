{
  description = "ragite: nightly Rust and cargo-fuzz";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
    in {
      devShells = nixpkgs.lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          # cargo fuzz builds the targets with nightly. The cargo-fuzz binary
          # is the subcommand; nightly rustc must be the cargo on PATH.
          rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
            toolchain.default.override { extensions = [ "rust-src" ]; });
        in {
          default = pkgs.mkShell {
            packages = [ rust pkgs.cargo-fuzz ];
          };
        });
    };
}
