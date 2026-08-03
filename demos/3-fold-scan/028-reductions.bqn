# ano: +/ Gold @ ^party, #/ ^party, +/ Gold @ !^party, and the empty live scope ^empty under every fold class.
# Semantics: a fold's scope may be a live alias mask; the fold itself is the unseeded left recurrence, seeded only where the descriptor declares an identity.
# Invariant: the sigiled scope and its negation partition the total; an identityless fold over the empty scope has no result row (A12) — the guard is 0<count, never a manufactured value.

gold  ← 100‿50‿20‿5‿0‿25‿7‿3
party ← 1‿1‿0‿1‿0‿0‿0‿0                    # the sidecar's live mask
empty ← 0‿0‿0‿0‿0‿0‿0‿0

LeftSum ← {+˜´⌽(0∾𝕩)}
NoRow   ← {0=≠𝕩 ? ⟨⟩ ; ⟨⌈˜´⌽𝕩⟩}

! 155 ≡ LeftSum party/gold
! 3 ≡ +´ party
! 55 ≡ LeftSum (¬party)/gold
! 210 ≡ 155+55                             # the partition law
! 0 ≡ LeftSum empty/gold                   # + declares identity 0
! 0 ≡ +´ empty                             # count declares 0
! 0 = ≠ NoRow empty/gold                   # max declares none: no result row
