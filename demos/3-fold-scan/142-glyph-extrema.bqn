# ano:  "sat" | "cow", "sat" & "cow", max/ Rune, min/ Rune, |\ Rune, &\ Rune over "genie"
# Semantics: BQN's ⌈ and ⌊ refuse characters outright, so the char instances of A9's Greater and Lesser step through code points — @ is the null character, c-@ is a character's code point, and @+n is the character at one. One declared step serves the direct dyad, the fold and the scan, so the three cannot diverge; the order is the code-point order.
# Invariant: the elementwise dyads are q's Greater and Lesser on strings; the folds are the left recurrence over that step; each fold is the last element of its scan; and the order has no identity, so an empty run yields no row rather than a code-point zero.

CharGreater ← {@+(𝕨-@)⌈𝕩-@}
CharLesser  ← {@+(𝕨-@)⌊𝕩-@}
LeftCharMax ← {CharGreater˜´⌽𝕩}
LeftCharMin ← {CharLesser˜´⌽𝕩}

! "sow" ≡ "sat" CharGreater¨ "cow"
! "cat" ≡ "sat" CharLesser¨ "cow"

! 'n' ≡ LeftCharMax "genie"
! 'e' ≡ LeftCharMin "genie"
! "ggnnn" ≡ CharGreater` "genie"
! "geeee" ≡ CharLesser` "genie"

# the fold is the last element of its own scan, on this carrier as on every other
! (LeftCharMax "genie") ≡ ¯1⊑CharGreater` "genie"
! (LeftCharMin "genie") ≡ ¯1⊑CharLesser` "genie"

# why the round trip exists at all: the primitives have no character instance
! "refused" ≡ 'a' ({𝕨⌈𝕩}⎊{"refused"}) 'b'

# no identity: the empty run has no result row, and nothing is manufactured to stand for one
NoRow ← {0=≠𝕩 ? ⟨⟩ ; ⟨LeftCharMax 𝕩⟩}
! 0 = ≠ NoRow ""
