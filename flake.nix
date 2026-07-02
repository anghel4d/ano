{
  description = "ano dev shell: BQN for verification, GHC for draft work";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cbqn # BQN, the verification language for all executable claims
            (pkgs.haskellPackages.ghcWithPackages (p: [ p.megaparsec ])) # draft interpreters, reader experiments
            pkgs.cabal-install
            pkgs.rlwrap # line editing for the bqn repl
          ];
        };
      });
    };
}
