# ano: fold/ rel'.Comp   fold/ (rel' & pred)   -- always per-source; fold/ col @ scope is always one scalar
# Semantics: ids group vals into fibers, the fold collapses each fiber to one value per selected target j; an empty fiber takes the fold's registered identity, and a fold with no identity drops the row — the §5 left-join-null rule extended from the dangling link to the empty fiber.
# Invariant: grouped counts agree with /⁼, the column follows the selection's target order, identity folds keep empty-fiber rows at the identity, avg drops them.

# Gamma. Inputs: 𝕨 ← ⟨F, hasId⟩ — F folds one fiber (list) to a scalar, hasId flags a registered identity; 𝕩 ← ⟨ids, vals, targets⟩ — ids the target in J per source (≠ids ≡ ≠vals), vals the per-source column, targets the selected ids in J.
# Output: ⟨keep, col⟩ — keep masks the targets whose row survives; col is one value per surviving target, in target order.
# Invariant: hasId=1 keeps every row (F of the empty fiber is the identity); hasId=0 drops empty-fiber rows before F runs.
Gamma ← {F‿hasId 𝕊 ids‿vals‿targets:
  fibers ← targets ⊏ (ids ∾ 1+⌈´targets) ⊔ vals
  keep ← hasId ∨ 0 < ≠¨fibers
  keep ⋈ F¨ keep/fibers
}

# r: I -> J with J = ↕4; targets 1 and 3 have empty fibers.
ids  ← 0‿0‿2‿2‿2
vals ← 3‿5‿2‿4‿9

# count (#/ rel'): identity 0; empty fibers count 0
! ⟨1‿1‿1‿1, 2‿0‿3‿0⟩ ≡ ⟨≠,1⟩ Gamma ids‿vals‿(↕4)
! 2‿0‿3 ≡ /⁼ids            # the grouped-counts idiom agrees on the ids' own range

# sum (+/ rel'.Comp): identity 0
! ⟨1‿1‿1‿1, 8‿0‿15‿0⟩ ≡ (+´)‿1 Gamma ids‿vals‿(↕4)

# any and all (|/ rel'.Comp, &/ rel'.Comp): identities false, true
flags ← 1‿0‿0‿0‿1
! ⟨1‿1‿1‿1, 1‿0‿1‿0⟩ ≡ (∨´)‿1 Gamma ids‿flags‿(↕4)
! ⟨1‿1‿1‿1, 0‿1‿0‿1⟩ ≡ (∧´)‿1 Gamma ids‿flags‿(↕4)

# avg (avg/ rel'.Comp): no identity; the empty-fiber rows drop out of the selection, no write lands
! ⟨1‿0‿1‿0, 4‿5⟩ ≡ ⟨+´÷≠, 0⟩ Gamma ids‿vals‿(↕4)

# alignment: the column follows the selection's target order, not id order
! ⟨1‿1, 3‿2⟩ ≡ ⟨≠,1⟩ Gamma ids‿vals‿(2‿0)
