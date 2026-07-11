# The proto and the three-layer spawn fill.
# ano: Soldier , spawn Marine
# Semantics: the proto (def Marine soldier=1 hp=100) is the registered archetype. Spawn fill is a three-layer lookup, per column: the proto's declared value, else the column's registered default, else the type's zero. The unique id column takes no layer — declared injectivity forces the fresh mint (1+max) — and the proto-role sym column records the archetype's noun.
# Invariant: each layer is witnessed by one column — soldier/hp (proto), ammo (default 5), gold (type zero 0) — and the mint extends the pairwise-distinct id column.

id      ← 3‿5
soldier ← 1‿0
hp      ← 40‿10
ammo    ← 7‿7                       # registered default: 5
gold    ← 100‿200                   # no proto value, no default: type zero
proto   ← ⟨"Grunt", "Grunt"⟩

# Soldier , spawn Marine   (one copy per selected source)
k ← +´soldier
id      ∾↩ (1+⌈´id)+↕k              # the mint: injectivity admits nothing else
soldier ∾↩ k⥊1                      # layer one: the proto's value
hp      ∾↩ k⥊100                    # layer one
ammo    ∾↩ k⥊5                      # layer two: the registered default
gold    ∾↩ k⥊0                      # layer three: the type zero
proto   ∾↩ k⥊<"Marine"              # the archetype's noun in the proto role

! id ≡ 3‿5‿6
! soldier ≡ 1‿0‿1
! hp ≡ 40‿10‿100
! ammo ≡ 7‿7‿5
! gold ≡ 100‿200‿0
! proto ≡ ⟨"Grunt", "Grunt", "Marine"⟩
