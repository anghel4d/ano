# Relationship hops (ex8).
# ano: Nord & mentor.TwoHanded > 80 , Gold += 1000
# ano: Student & mentor.Dead , -Mentored
# ano: Soldier & leader.AtWar , Morale -= 20
# Semantics: rel.Comp gathers Comp at the target ID held in rel — one indexed read; an absent or dangling link fails the predicate (left-join-null).
# Invariant: entities with no mentor never pass a mentor.* predicate; the gather is index-from-column, guarded before use.

nil ← ¯1                            # absent-link sentinel
mentor    ← 2‿nil‿4‿2‿nil‿1         # entity → mentor id
hasMentor ← mentor ≥ 0
leader    ← 0‿0‿0‿1‿0‿1             # entity → leader id (rows 0..1 double as leaders)

nord      ← 1‿1‿1‿1‿0‿1
twoHanded ← 90‿20‿85‿70‿81‿30
student   ← 1‿1‿1‿1‿0‿1
dead      ← 0‿0‿1‿0‿1‿0
mentored  ← 1‿0‿1‿1‿0‿1
soldier   ← 0‿0‿1‿1‿1‿1
atWar     ← 1‿0‿0‿0‿0‿0
gold      ← 0‿0‿0‿0‿0‿0
morale    ← 100‿100‿100‿100‿100‿100

mTH ← (0⌈mentor)⊏twoHanded          # guarded gather; the ⌈ clamp is dead weight where hasMentor is 0
gold +↩ 1000 × nord ∧ hasMentor ∧ mTH>80

mDead ← (0⌈mentor)⊏dead
mentored ∧↩ ¬ student ∧ hasMentor ∧ mDead

morale -↩ 20 × soldier ∧ leader⊏atWar

! 1000‿0‿1000‿1000‿0‿0 ≡ gold
! 0‿0‿0‿0‿0‿1 ≡ mentored
! 100‿100‿80‿100‿80‿100 ≡ morale
# rows 1 and 4 have no mentor: the dangling link failed both mentor.* predicates
"left-join-null" ! 0‿0 ≡ 1‿4⊏gold
