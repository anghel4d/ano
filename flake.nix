{
  description = "ano dev shell: BQN for verification, the demo languages (Haskell, Erlang, OCaml, APL), and Lean4 for proofs";

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
            pkgs.rlwrap # line editing for the bqn and apl repls

            # Demo languages beyond BQN (demos/demos.md): Erlang, Haskell, OCaml.
            pkgs.beamPackages.erlang
            (pkgs.haskellPackages.ghcWithPackages (p: [ p.megaparsec ])) # draft interpreters, reader experiments
            pkgs.cabal-install

            # Bootstrap front-end candidates (spec: APL, Haskell, or OCaml).
            pkgs.gnuapl # .apl.history is a GNU APL repl history
            pkgs.ocaml
            pkgs.dune_3
            pkgs.ocamlPackages.findlib
            pkgs.ocamlPackages.utop
            pkgs.ocamlPackages.menhir

            pkgs.lean4 # proofs/foundations.md, pending mechanization
          ];
        };
      });

      # `nix flake check` runs every demo under demos/ through check.sh.
      checks = forAll (pkgs: {
        demos = pkgs.runCommand "ano-demos" { nativeBuildInputs = [ pkgs.cbqn ]; } ''
          bash ${self}/demos/check.sh
          touch $out
        '';
      });
    };
}
