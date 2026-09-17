# Registry in demos/registries/ (relative-path form).
# ano: Alive , Hp += 10
# Semantics: same world and heal as 129-beside; here the fixture lives in demos/registries/ and
# Invariant: living rows gain exactly 10, the dead row is untouched.

alive ← 1‿0‿1
hp    ← 100‿100‿100

hp +↩ 10 × alive

! 110‿100‿110 ≡ hp
