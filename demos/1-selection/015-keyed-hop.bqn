# The keyed hop: rel;unique⁻¹.
# ano: Nord & mentor.Dead , Gold += 100
#      Cursed , ~
#      Nord & mentor.Dead , -Trained
#      Frenzy.targets' , +Frenzied
#      Frenzy , Reach = #/ targets'
# Semantics: `unique id` is declared injectivity, and an injective column is invertible on its image — exactly what a hop needs. The keyed hop is one ⊐ against the id column with one found-guard; the keyed srel's image is membership through the same column; the gamma over keyed fibers translates keys to live rows and drops dead members.
# Invariant: the ids are primes, so no positional reading agrees anywhere; the same hop is sound before and after the despawn with no fault path (not-found is dangling is dead).

id       ← 7‿11‿13‿17‿19            # pairwise-distinct, the load-checked injectivity
mentor   ← ¯1‿7‿7‿13‿17             # values are id KEYS, never row positions
targets  ← ⟨13‿17, ⟨⟩, ⟨⟩, ⟨⟩, ⟨⟩⟩  # keyed srel: fiber values are id keys
dead     ← 1‿0‿0‿0‿0
cursed   ← 0‿1‿0‿0‿0
nord     ← 1‿1‿1‿1‿1
trained  ← 1‿1‿1‿1‿1
gold     ← 0‿0‿0‿0‿0
frenzy   ← 1‿0‿0‿0‿0
frenzied ← 0‿0‿0‿0‿0
reach    ← 0‿0‿0‿0‿0

# statement 1: Nord & mentor.Dead , Gold += 100   (keyed hop, pre-despawn)
f     ← id⊐mentor                   # rel;unique⁻¹: slot of each key, ≠id when not found
alive ← f<≠id
mDead ← ((≠id)|f)⊏dead
gold +↩ 100 × nord∧alive∧mDead

! gold ≡ 0‿100‿100‿0‿0              # both students of the dead master (ids 11, 13)

# statement 2: Cursed , ~   (id 11 leaves; the id column filters like any column)
keep ← ¬cursed
id‿mentor‿targets‿dead‿cursed‿nord‿trained‿gold‿frenzy‿frenzied‿reach ↩ keep⊸/¨ id‿mentor‿targets‿dead‿cursed‿nord‿trained‿gold‿frenzy‿frenzied‿reach

! id ≡ 7‿13‿17‿19

# statement 3: Nord & mentor.Dead , -Trained   (same hop, shifted world, same guard)
f     ↩ id⊐mentor
alive ↩ f<≠id
mDead ↩ ((≠id)|f)⊏dead
trained ∧↩ ¬ nord∧alive∧mDead

! trained ≡ 1‿0‿1‿1                 # id 13 alone: its master (id 7) is dead; id 11 is gone

# statement 4: Frenzy.targets' , +Frenzied   (image: membership through the key column)
frenzied ∨↩ id ∊ ∾⟨⟨⟩⟩∾frenzy/targets

! frenzied ≡ 0‿1‿1‿0                # ids 13 and 17, each once

# statement 5: Frenzy , Reach = #/ targets'   (gamma: fiber keys → live rows, dead drop)
live ← {k←id⊐𝕩 ⋄ (k<≠id)/k}¨targets
reach ↩ reach + frenzy × ≠¨live     # masked write; off-mask rows keep 0

! reach ≡ 2‿0‿0‿0
