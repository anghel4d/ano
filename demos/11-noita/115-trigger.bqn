
pos ← ⟨40‿5, 60‿4, 20‿30, 40‿6, 61‿5, 0‿0⟩
sb ← 1‿1‿1‿0‿0‿0 ⋄ trig ← 1‿0‿0‿0‿0‿0 ⋄ fb ← 6⥊0
enemy ← 0‿0‿0‿1‿1‿0 ⋄ health ← 0‿0‿0‿25‿25‿0 ⋄ dmg ← 3‿3‿3‿0‿0‿0
keys ← ↕6 ⋄ parent ← 6⥊¯1

D ← {√+´×˜𝕨-𝕩}
bIdx ← /sb ⋄ eIdx ← /enemy
mask ← 8 > (bIdx⊏pos) D⌜ eIdx⊏pos
! (3‿2⥊1‿0‿0‿1‿0‿0) ≡ mask
hit ← 6⥊0
hit ↩ (∨´˘mask)⌾(bIdx⊸⊏) hit
hit ↩ (∨˝ mask)⌾(eIdx⊸⊏) hit
! 1‿1‿0‿1‿1‿0 ≡ hit

health ↩ health - 3 × enemy∧hit
! 0‿0‿0‿22‿22‿0 ≡ health

keep ← ¬ sb∧hit∧¬trig
F ← keep⊸/
keys ↩ F keys ⋄ sb ↩ F sb ⋄ trig ↩ F trig ⋄ fb ↩ F fb ⋄ dmg ↩ F dmg
hit ↩ F hit ⋄ enemy ↩ F enemy ⋄ health ↩ F health ⋄ pos ↩ F pos ⋄ parent ↩ F parent

sel ← sb∧trig∧hit
newPos ← sel/pos ⋄ newPar ← /sel ⋄ newKey ← (1+⌈´keys)+↕≠newPar
G ← (¬sel)⊸/
keys ↩ (G keys)∾newKey ⋄ pos ↩ (G pos)∾newPos ⋄ parent ↩ (G parent)∾newPar
sb ↩ (G sb)∾0¨newPar ⋄ trig ↩ (G trig)∾0¨newPar ⋄ fb ↩ (G fb)∾1¨newPar
dmg ↩ (G dmg)∾3¨newPar ⋄ hit ↩ (G hit)∾0¨newPar
enemy ↩ (G enemy)∾0¨newPar ⋄ health ↩ (G health)∾0¨newPar

! 2‿3‿4‿5‿6 ≡ keys
! 1‿0‿0‿0‿0 ≡ sb
! 0‿0‿0‿0‿1 ≡ fb
! 0‿1‿1‿0‿0 ≡ hit
! 0‿22‿22‿0‿0 ≡ health
! 3‿0‿0‿0‿3 ≡ dmg
! ¯1‿¯1‿¯1‿¯1‿0 ≡ parent
! ⟨20‿30, 40‿6, 61‿5, 0‿0, 40‿5⟩ ≡ pos
