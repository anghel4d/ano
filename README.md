# ano

あの — forays into a new embedded scripting language, and a language-research foray into its accidental isomorphism with Japanese grammar.

How ano works in one line of pseudocode:
```haskell
that-one-over-there , do-this
```

## What it is

- Rules select entity sets by description: you name the things by what they are (hostile, Nord, rich, in Whiterun) and the host resolves which rows that is.
- Statements compile to bulk column operations over the engine's structure-of-arrays store, not loops over objects.
- The toolchain is in Rust (the `steel` compiler and the `kore` editor); compiled programs run on CBQN, a BQN runtime written in C.
- Every documented example is an executable test. The demos under `demos/` are the suite: 73 BQN witnesses and 213 Steel checks (80 Japanese/ASCII twin comparisons, 89 tick classes, 35 refusals, 6 accepts, 3 witnesses) pass, and 141 quarantined cases are reported as visible skips; 289 Rust tests cover the compiler and editor; `proofs/` holds the Lean semantic kernel.

Start with the [tour](tour.md), then the [manual and language notes](docs/).

## Build and test

```sh
nix flake check
```

This builds the Rust toolchain and runs the Lean proofs, every BQN demo, and the Steel battery. Without Nix, with `cargo` and `bqn` on the path: `bash demos/check.sh && bash src/check-ano.sh && cargo test --workspace`.

Public for reference. All rights reserved; see [LICENSE](LICENSE).
