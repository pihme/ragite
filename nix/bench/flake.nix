{
  description = "ragite: Python harness for the model benchmark";

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
          packages = with pkgs; [ (python312.withPackages (ps: [ ps.pip ps.setuptools ps.wheel ])) gnumake ];
            inputsFrom = [ sqlite.devShells.${system}.default ];
            shellHook = ''

              export PIP_PREFIX=$PWD/.pip
              export PATH="$PIP_PREFIX/bin:$PATH"
              py=$(python3 -c 'import sys; print("%d.%d" % sys.version_info[:2])')
              export PYTHONPATH="$PIP_PREFIX/lib/python$py/site-packages''${PYTHONPATH:+:$PYTHONPATH}"

            '';
        };
      });
    };
}
