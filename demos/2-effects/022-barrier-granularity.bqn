# ano: Nord & Dead , spawn Ghost
#      ~
# Semantics: barrier one gathers Nord & Dead, spawns one Ghost per selected row, and commits; barrier two's gather observes that commit, but the `~` despawn applies to the mask saved at barrier one's gather — never a re-gather — so ghost rows born after the gather are exempt even though they would match the predicate.

id    ← 0‿1‿2‿3‿4‿5
nord  ← 1‿1‿0‿1‿0‿0
dead  ← 1‿0‿0‿1‿0‿1
ghost ← 0‿0‿0‿0‿0‿0

# barrier one: gather, save the mask (as ids: a mask, not a snapshot), spawn, commit
sel   ← nord∧dead
saved ← sel/id
! saved ≡ 0‿3

n ← +´sel                    # ghosts of dead Nords: they keep Nord and Dead
id    ∾↩ 6+↕n
nord  ∾↩ 1¨↕n
dead  ∾↩ 1¨↕n
ghost ∾↩ 1¨↕n
! 8 ≡ ≠id                    # barrier two's gather observes barrier one's scatter

# barrier two: the `~` despawns the saved ids
keep ← ¬id∊saved
idC‿nordC‿ghostC ← keep⊸/¨ id‿nord‿ghost

! idC    ≡ 1‿2‿4‿5‿6‿7
! nordC  ≡ 1‿0‿0‿0‿1‿1
! ghostC ≡ 0‿0‿0‿0‿1‿1
! 2 ≡ +´ghostC

keepW ← ¬nord∧dead
idW‿ghostW ← keepW⊸/¨ id‿ghost

! idW ≡ 1‿2‿4‿5
! 0 ≡ +´ghostW               # re-gathering would kill the ghosts
! ¬idC ≡ idW                 # saved-mask and re-gather genuinely differ
