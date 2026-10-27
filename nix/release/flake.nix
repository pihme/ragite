{
  description = "ragite: release build of the extension against an old glibc baseline";

  inputs.sqlite.url = "path:../sqlite";
  # Same nixpkgs as the test shells, so one Dependabot bump moves both.
  inputs.nixpkgs.follows = "sqlite/nixpkgs";

  outputs = { self, nixpkgs, sqlite }:
    let
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];
      forAll = f: nixpkgs.lib.genAttrs systems (system:
        f system nixpkgs.legacyPackages.${system});
    in {
      devShells = forAll (system: pkgs: {
        # cargo zigbuild links against zig's glibc stubs for the version in the
        # target triple (x86_64-unknown-linux-gnu.2.34), not against Nix's glibc.
        default = pkgs.mkShell {
          packages = with pkgs; [ zig cargo-zigbuild binutils ];
          inputsFrom = [ sqlite.devShells.${system}.default ];
        };
      });
    };
}
