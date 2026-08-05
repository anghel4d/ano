# ano: 死んだ北が、幽霊を生み、消える。
# ano: あの Nord & Dead が spawn Wraith
# ano: ~
# ano: Nord & Dead , spawn Wraith
# ano: ~
# Invariant: after both barriers the dead Nords are gone, every other original row remains, and one ghost per dead Nord survives.

type ← ⟨"nord","nord","imperial","nord","nord","khajiit"⟩
nord ← 1‿1‿0‿1‿1‿0
dead ← 1‿0‿0‿1‿0‿0

sel ← nord∧dead                # gather: the anaphora saves the mask
k ← +´sel
type ∾↩ k⥊<"ghost"             # barrier one: spawn Ghost per selected row
nord ∾↩ k⥊0
dead ∾↩ k⥊0

keep ← ¬sel∾k⥊0                # barrier two: the saved mask, zero-padded over the new rows
type ↩ keep/type
nord ↩ keep/nord
dead ↩ keep/dead

"ex49: post-state types"!(⟨"nord","imperial","nord","khajiit","ghost","ghost"⟩≡type)
"ex49: ghosts survive the despawn"!(2=+´"ghost"⊸≡¨type)
"ex49: no dead Nord remains"!(0=+´nord∧dead)
