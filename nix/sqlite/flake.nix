{
  description = "ragite: SQLite CLI and library for the extension";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.reference-env.url = "path:../reference-env";

  outputs = { self, nixpkgs, reference-env }:
    let
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];
      forAll = f: nixpkgs.lib.genAttrs systems (system:
        f system nixpkgs.legacyPackages.${system});
    in {
      devShells = forAll (system: pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ sqlite pkg-config ];
            inputsFrom = [ reference-env.devShells.${system}.default ];
            # cgo asks for libsqlite3.so.0 by file name. gcc searches
            # LIBRARY_PATH, and the loader needs LD_LIBRARY_PATH afterwards.
            shellHook = ''
              export LIBRARY_PATH=${pkgs.sqlite.out}/lib''${LIBRARY_PATH:+:$LIBRARY_PATH}
              export LD_LIBRARY_PATH=${pkgs.sqlite.out}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
            '';
        };
      });
    };
}
