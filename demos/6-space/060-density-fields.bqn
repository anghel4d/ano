# Density fields (ex32, spec §22). Modeled on an 8-cell strip; the doc's 64 64 is the same statement at scale.
# Ano: 64 64 , spawn Tree * Density
#      64 64 & Fertility > 0 , spawn Crop * Fertility
# Semantics: replicate turns a per-cell count field into a population — each cell contributes count copies of the spawn row; the predicate filters the cells BEFORE the counts replicate, so the mask and the count column stay aligned.
# Invariant: spawned rows are exactly the hand-computed cell-index replication; the world grows by the field total over the selected cells; filtering after replication is length-misaligned and is never what the statement means.

# world as columns: no entities yet, cells carry the fields
density ← ⟨0,2,1,0,3,0,1,2⟩
fert    ← ⟨2,0,¯1,3,0,1,4,0⟩
worldPos  ← ⟨⟩
worldType ← ""

# spawn Tree * Density : each cell spawns density copies at its own position
treePos ← density/↕8
worldPos  ∾↩ treePos
worldType ∾↩ 'T'¨treePos
! treePos ≡ ⟨1,1,2,4,4,4,6,7,7⟩
! 9 = +´density
! 9 = ≠worldPos

# & Fertility > 0 , spawn Crop * Fertility : the mask selects cells first,
# THEN the surviving counts replicate — (mask/fert)/mask/cells, never mask/fert/cells
mask ← 0<fert
! mask ≡ ⟨1,0,0,1,0,1,1,0⟩
cropPos ← (mask/fert)/mask/↕8
worldPos  ∾↩ cropPos
worldType ∾↩ 'C'¨cropPos
! cropPos ≡ ⟨0,0,3,3,3,5,6,6,6,6⟩
! 10 = +´mask/fert
! (mask/fert) ≡ ⟨2,3,1,4⟩

# post-state: 19 rows, trees then crops, positions exact
! 19 = ≠worldPos
! worldPos  ≡ ⟨1,1,2,4,4,4,6,7,7⟩∾⟨0,0,3,3,3,5,6,6,6,6⟩
! worldType ≡ "TTTTTTTTTCCCCCCCCCC"

# Dependent-sum witness: entity growth is the selected count total; the eight source cells and both source fields remain eight rows.
cells ← ↕8
sourceCount ← +´density
sourceCount +↩ +´mask/fert
! 8 = ≠cells
! 8 = ≠density
! 8 = ≠fert
! 19 = sourceCount
! 19 = ≠worldPos
! cells ≡ ↕8
