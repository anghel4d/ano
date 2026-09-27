# ano: 北全員の金の総和。
# ano: +/ Gold @ Nord
# Invariant: the fold equals the hand-summed Nord gold and has scalar (rank-0) shape.

nord ← 1‿0‿1‿1‿0‿1
gold ← 100‿250‿300‿50‿999‿150
total ← +´nord/gold
"ex45: scoped fold value"!(600=total)
"ex45: @ after a fold is one scalar"!(⟨⟩≡≢total)
