{
  description = "ragite: Python, Node.js and Go loaders for make test";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.sqlite.url = "path:../sqlite";

  outputs = { self, nixpkgs, sqlite }:
    let
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];
      forAll = f: nixpkgs.lib.genAttrs systems (system:
        f system nixpkgs.legacyPackages.${system});
    in {
      devShells = forAll (system: pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ python312 nodejs_24 go gnumake ];
            inputsFrom = [ sqlite.devShells.${system}.default ];
        };
      });
    };
}
