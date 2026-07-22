
density ← ⟨0,2,1,0,3,0,1,2⟩
fert    ← ⟨2,0,¯1,3,0,1,4,0⟩
worldPos  ← ⟨⟩
worldType ← ""

treePos ← density/↕8
worldPos  ∾↩ treePos
worldType ∾↩ 'T'¨treePos
! treePos ≡ ⟨1,1,2,4,4,4,6,7,7⟩
! 9 = +´density
! 9 = ≠worldPos

mask ← 0<fert
! mask ≡ ⟨1,0,0,1,0,1,1,0⟩
cropPos ← (mask/fert)/mask/↕8
worldPos  ∾↩ cropPos
worldType ∾↩ 'C'¨cropPos
! cropPos ≡ ⟨0,0,3,3,3,5,6,6,6,6⟩
! 10 = +´mask/fert
! (mask/fert) ≡ ⟨2,3,1,4⟩

! 19 = ≠worldPos
! worldPos  ≡ ⟨1,1,2,4,4,4,6,7,7⟩∾⟨0,0,3,3,3,5,6,6,6,6⟩
! worldType ≡ "TTTTTTTTTCCCCCCCCCC"

cells ← ↕8
sourceCount ← +´density
sourceCount +↩ +´mask/fert
! 8 = ≠cells
! 8 = ≠density
! 8 = ≠fert
! 19 = sourceCount
! 19 = ≠worldPos
! cells ≡ ↕8
