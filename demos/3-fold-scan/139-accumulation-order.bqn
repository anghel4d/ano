# ano:  +/ Drift, +\ Drift, avg/ Drift, avg\ Drift over 1, 1e100, -1e100
# Semantics: an Ano fold is the unseeded LEFT accumulation (A13). BQN's ´ folds right, so the emitted form is F˜´⌽x; BQN's ` is already left and needs no correction. Over this column the two orders answer 0 and 1, which is the whole point of the fixture — the 1 is lost into 1e100 and the pair then cancels.
# Invariant: the left sum is 0 where the right sum is 1; the scan's last element is the fold; and the mean's finish accumulates in the machine's order, so avg/ is the last prefix of avg\ rather than the backend's own mean.

drift ← 1‿1e100‿¯1e100

LeftSum ← {+˜´⌽(0∾𝕩)}          # exactly what Steel renders for +/
Mean    ← {(LeftSum 𝕩)÷≠𝕩}     # and for avg/

! 0 ≡ LeftSum drift
! 1 ≡ +´ drift                 # the right fold, shown only to be refused
! 1‿1e100‿0 ≡ +` drift

# the law: the fold is the last element of its own scan, for the sum and for the mean
! (LeftSum drift) ≡ ¯1⊑+` drift
! 0 ≡ Mean drift
! 1‿5e99‿0 ≡ (+`drift)÷+`1¨drift
! (Mean drift) ≡ ¯1⊑(+`drift)÷+`1¨drift

# the extrema carry no identity and manufacture no infinity; here they are simply the ends
! 1e100 ≡ ⌈˜´⌽drift
! ¯1e100 ≡ ⌊˜´⌽drift
