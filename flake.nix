{
  description = "Ano: Rust compiler and editor, CBQN runtime, demo languages, and Lean proofs";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # nixpkgs cbqn ships bin/BQN + bin/cbqn everywhere but adds the lowercase
      # bqn only on Linux; steel spawns `bqn`, so darwin gets the name declaratively.
      bqnPkgs = pkgs: [ pkgs.cbqn ] ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [
        (pkgs.runCommand "bqn-name" { } "mkdir -p $out/bin && ln -s ${pkgs.cbqn}/bin/BQN $out/bin/bqn")
      ];

      # crates.io deps vendored from the lockfile; the sandboxed checks stay offline.
      cargoVendor = pkgs: pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
    in
    {
      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = bqnPkgs pkgs ++ [
            pkgs.rlwrap # line editing for the bqn and apl repls

            # Rust compiler and editor.
            pkgs.rustc
            pkgs.cargo

            # Demo languages beyond BQN (demos/demos.md): Erlang, Haskell, OCaml.
            pkgs.beamPackages.erlang
            (pkgs.haskellPackages.ghcWithPackages (p: [ p.megaparsec ])) # draft interpreters, reader experiments
            pkgs.cabal-install

            # OCaml demonstrations and reader experiments.
            pkgs.ocaml
            pkgs.dune_3
            pkgs.ocamlPackages.findlib
            pkgs.ocamlPackages.utop
            pkgs.ocamlPackages.menhir

            pkgs.lean4 # proofs/Ano/, the machine-checked semantic kernel
          ] ++ nixpkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.gnuapl # .apl.history is a GNU APL repl history; no aarch64-darwin build in nixpkgs
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
        demos = pkgs.runCommand "ano-demos" { nativeBuildInputs = bqnPkgs pkgs; } ''
          bash ${self}/demos/check.sh
          touch $out
        '';
        steel = pkgs.runCommand "ano-steel-demos" { nativeBuildInputs = bqnPkgs pkgs ++ [ pkgs.rustc pkgs.cargo pkgs.stdenv.cc ] ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.apple-sdk ]; } ''
          cp ${self}/Cargo.toml Cargo.toml && cp ${self}/Cargo.lock Cargo.lock && cp -r ${self}/steel steel && cp -r ${self}/kore kore
          cp -r ${self}/src src && cp -r ${self}/demos demos && cp -r ${self}/todo todo
          chmod -R +w steel kore src demos todo
          export CARGO_HOME=$PWD/.cargo
          mkdir -p .cargo
          printf '[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "%s"\n' ${cargoVendor pkgs} > .cargo/config.toml
          cargo build --release --offline --workspace
          STEEL=$PWD/target/release/steel bash src/check-ano.sh
          touch $out
        '';
      });
    };
}
