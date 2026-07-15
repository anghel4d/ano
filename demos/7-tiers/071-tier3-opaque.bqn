# 39 - Registered capabilities: native matrix algebra and host dispatch
# ano: Node , OutDeg = +/ Adj@row
# ano: Hostile , shortestPath via Adj
# ano: ^cursor , runBehaviorTree
# Semantics: one registered interface exposes a boolean matrix field to native row folds; other registered interfaces dispatch graph and behavior carriers to host callables. These are capabilities and signatures, not tiers.
# Invariant: the matrix-view fold matches hand-computed out-degrees; the dispatch envelope is exactly the selected identities; opaque payloads are bit-identical after selection.

# world: five nodes; Adj as bits, opaque payloads as boxed values
adj ← >⟨0‿1‿1‿0‿0
        0‿0‿1‿0‿0
        1‿0‿0‿1‿1
        0‿0‿0‿0‿0
        0‿1‿0‿0‿1⟩
hostile ← 0‿1‿0‿1‿1
cursor  ← 0‿0‿0‿1‿0
btree   ← ⟨"seek", "flee", "guard", "patrol", "idle"⟩   # opaque: ano never looks inside

# statement 1: matrix interface — out-degree is a native row fold
outdeg ← +´˘adj
! 2‿1‿3‿0‿2 ≡ outdeg

# statement 2: graph interface — no native expression is registered; the envelope is the selection and the value goes to the host whole
env ← /hostile
! 1‿3‿4 ≡ env
! adj ≡ >⟨0‿1‿1‿0‿0, 0‿0‿1‿0‿0, 1‿0‿0‿1‿1, 0‿0‿0‿0‿0, 0‿1‿0‿0‿1⟩   # dispatch reads Adj, never rewrites it

# statement 3: opaque view — ^cursor selects one identity, the payload crosses the wall untouched
! ⟨3⟩ ≡ /cursor
! "patrol" ≡ ⊑(/cursor)⊏btree
! btree ≡ ⟨"seek", "flee", "guard", "patrol", "idle"⟩                # selection is the only native act
