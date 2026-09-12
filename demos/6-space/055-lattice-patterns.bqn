
x ← ⌊(↕64)÷8
y ← 8|↕64

wheat     ← 0=2|x+y
barley    ← 1=2|x+y
pillar    ← x=y
buildable ← 8>x+y
fence     ← 0=3|x

! (8‿8⥊wheat)     ≡ 0=2|+⌜˜↕8
! (8‿8⥊barley)    ≡ 1=2|+⌜˜↕8
! (8‿8⥊pillar)    ≡ =⌜˜↕8
! (8‿8⥊buildable) ≡ 8>+⌜˜↕8
! (8‿8⥊fence)     ≡ (0=3|↕8)×⌜8⥊1

! ∧´wheat≠barley
! 32 = +´wheat
! 32 = +´barley

! 8 = +´pillar
! (9×↕8) ≡ /pillar

! 36 = +´buildable

! 24 = +´fence
! (∾⟨↕8, 24+↕8, 48+↕8⟩) ≡ /fence

shape ← 8‿8
ground ← ↕shape
! shape ≡ ≢ground
! shape ≡ ≢8‿8⥊wheat
! shape ≡ ≢8‿8⥊barley
! shape ≡ ≢8‿8⥊pillar
! shape ≡ ≢8‿8⥊buildable
! shape ≡ ≢8‿8⥊fence
