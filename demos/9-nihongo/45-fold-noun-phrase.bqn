# Ex45 — Japanese fold noun phrase: の gathers, 総和 reduces.
# ano: 北全員の金の総和。
# ano: +/ Gold @ Nord
# Semantics: a pure noun phrase names a scoped-global fold — after the gamma decision, @ after a fold always yields one scalar over the scope, never a group.
# Invariant: the fold equals the hand-summed Nord gold and has scalar (rank-0) shape.

nord ← 1‿0‿1‿1‿0‿1
gold ← 100‿250‿300‿50‿999‿150
total ← +´nord/gold
"ex45: scoped fold value"!(600=total)
"ex45: @ after a fold is one scalar"!(⟨⟩≡≢total)
