# Registry beside the twin (bare-name form).
# ano: Alive , Hp += 10
# Semantics: heal every living entity by 10; the fixture r1-beside.reg sits in this directory.
# Invariant: living rows gain exactly 10, the dead row is untouched.

alive ← 1‿0‿1
hp    ← 100‿100‿100

hp +↩ 10 × alive

! 110‿100‿110 ≡ hp
