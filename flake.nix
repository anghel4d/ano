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

            # anoc, the ano -> BQN transpiler in src/ (C23).
            pkgs.gcc
            pkgs.gnumake

            # Steel — the Rust reference implementation (steel/ and kore/, one workspace).
            pkgs.rustc
            pkgs.cargo

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

            pkgs.lean4 # proofs/Ano/, the machine-checked semantic kernel
          ];

          # kore's E hop and every editor-shaped fallback land on a real editor,
          # never Debian's vim.tiny. Only when the user hasn't chosen one.
          shellHook = ''
            export EDITOR="''${EDITOR:-nvim}"
          '';
        };
      });

      # `nix flake check` runs the Lean kernel, every demo under demos/ through
      # check.sh, and every .ano twin through steel (src/check-ano.sh).
      checks = forAll (pkgs: {
        proofs = pkgs.runCommand "ano-proofs" { nativeBuildInputs = [ pkgs.lean4 ]; } ''
          cp -r ${self}/proofs proofs
          chmod -R +w proofs
          cd proofs
          lake build
          touch $out
        '';
        demos = pkgs.runCommand "ano-demos" { nativeBuildInputs = [ pkgs.cbqn ]; } ''
          bash ${self}/demos/check.sh
          touch $out
        '';
        anoc = pkgs.runCommand "ano-anoc-build" { nativeBuildInputs = [ pkgs.gcc pkgs.gnumake ]; } ''
          # The C anoc predates typed registries, which now permeate the corpus (Steel is
          # the reference, bootstrap ruling 2026-07-10; Cano re-derives from verified
          # Steel): it must still build, and checks.steel runs the demos.
          cp -r ${self}/src src && chmod -R +w src
          make -C src anoc
          touch $out
        '';
        steel = pkgs.runCommand "ano-steel-demos" { nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.gcc pkgs.cbqn ]; } ''
          cp ${self}/Cargo.toml Cargo.toml && cp -r ${self}/steel steel && cp -r ${self}/kore kore
          cp -r ${self}/src src && cp -r ${self}/demos demos
          chmod -R +w steel kore src demos
          export CARGO_HOME=$PWD/.cargo
          cargo build --release --offline --workspace
          STEEL=$PWD/target/release/steel bash src/check-ano.sh
          touch $out
        '';
      });
    };
}
