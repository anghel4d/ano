# 39 - Tier 3 opaque: matrix view is native algebra, graph and behaviour-tree views are select-and-dispatch
# ano: Node , OutDeg = +/ Adj@row
# ano: Hostile , shortestPath via Adj
# ano: ^cursor , runBehaviorTree
# Semantics: the same bits viewed as a matrix sit in Tier 1 (row folds are native); viewed as a graph or a behaviour tree no operator in the calculus applies, so ano models only the envelope — the selection handed to the host — and never inspects the value.
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

# statement 1: matrix view, Tier 1 — out-degree is a native row fold
outdeg ← +´˘adj
! 2‿1‿3‿0‿2 ≡ outdeg

# statement 2: graph view, Tier 3 — no native expression; the envelope is the selection, the value goes to the host whole
env ← /hostile
! 1‿3‿4 ≡ env
! adj ≡ >⟨0‿1‿1‿0‿0, 0‿0‿1‿0‿0, 1‿0‿0‿1‿1, 0‿0‿0‿0‿0, 0‿1‿0‿0‿1⟩   # dispatch reads Adj, never rewrites it

# statement 3: opaque view — ^cursor selects one identity, the payload crosses the wall untouched
! ⟨3⟩ ≡ /cursor
! "patrol" ≡ ⊑(/cursor)⊏btree
! btree ≡ ⟨"seek", "flee", "guard", "patrol", "idle"⟩                # selection is the only native act
