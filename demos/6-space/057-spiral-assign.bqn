# Assign spiral to existing entities (ex29, spec §19).
# Ano: Coin , pos = Player.pos + polar(index, index * 137.5)
# Semantics: the selection's own order supplies index; polar zips index with angle into one pair per selected entity — a column aligned to the selection, scattered back onto pos at the selected rows only.
# Invariant: polar consumes ≠Coin index-angle pairs (never a catenated flat 2·≠Coin list); coin pos match hand-computed spiral points; non-coin pos are untouched.

playerPos ← 100‿200

# world as columns: 8 entities, 5 coins
coin ← ⟨1,0,1,1,0,1,0,1⟩
pos  ← ⟨0‿0, 10‿10, 0‿0, 0‿0, 20‿20, 0‿0, 30‿30, 0‿0⟩

# the zip fix: index-angle PAIRS via ⋈¨, one pair per selected coin
n ← +´coin
pairs ← (↕n) ⋈¨ 137.5×↕n
! 5 = ≠pairs
! 10 = ≠(↕n)∾137.5×↕n      # the old catenated form: a flat 10-list, wrong shape for polar
! (⊑pairs)  ≡ 0‿0
! (1⊑pairs) ≡ 1‿137.5

d ← π÷180
Polar ← {r‿t: r×(•math.Cos t×d)⋈•math.Sin t×d}
spiral ← playerPos⊸+¨ Polar¨ pairs

# scatter the aligned column back onto the coin rows
pos ↩ spiral⌾((coin=1)⊸/) pos

# non-coins untouched
! ⟨10‿10, 20‿20, 30‿30⟩ ≡ (¬coin)/pos

# coins match hand-computed spiral points: r=index, theta=index×137.5 degrees
! (⊑pos) ≡ 100‿200
! ∧´ 1e¯9 > | (2⊑pos) - 100‿200+⟨¯0.737277336810124,  0.6755902076156604⟩
! ∧´ 1e¯9 > | (3⊑pos) - 100‿200+⟨0.17431148549531578, ¯1.992389396183491⟩
! ∧´ 1e¯9 > | (5⊑pos) - 100‿200+⟨1.826284287026163,   2.380060020873705⟩
! ∧´ 1e¯9 > | (7⊑pos) - 100‿200+⟨¯3.9392310120488325, ¯0.6945927106677191⟩
! 8 = ≠pos

# Lineage witness: the derived spiral has one value per selected entity and scatters back to the eight-row entity domain.
entityDomain ← ↕8
selectedDomain ← /coin
! 8 = ≠entityDomain
! 5 = ≠selectedDomain
! 5 = ≠spiral
! 8 = ≠pos
