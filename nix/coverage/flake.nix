{
  description = "ragite: cargo-llvm-cov";

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
          # The nixpkgs rustc segfaults on exit under -C instrument-coverage.
          # This is the rustup build, with the matching llvm-tools-preview.
          # It is not inputsFrom the reference env: that would put the
          # nixpkgs rustc on PATH as well.
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "clippy" "rustfmt" "llvm-tools-preview" ];
          };
          # Its test suite runs against the nixpkgs rustc, which is the crash
          # above, so the package does not build with checks on.
          cov = pkgs.cargo-llvm-cov.overrideAttrs (_: { doCheck = false; });
        in {
          default = pkgs.mkShell {
            packages = [ rust cov pkgs.sqlite pkgs.pkg-config ];
          };
        });
    };
}
