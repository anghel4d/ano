import Ano.Effects

namespace Ano.NegativeWitness

/-- Two nominal habitats backed by the same raw finite carrier. -/
structure EntitySite where
  raw : Fin 4

/-- Ground is intentionally nominally distinct from the entity habitat. -/
structure GroundSite where
  raw : Fin 4

def entityLayout : Layout 4 EntitySite where
  toFun := EntitySite.mk
  invFun := EntitySite.raw
  leftInv := by intro i; rfl
  rightInv := by intro e; cases e; rfl

def groundLayout : Layout 4 GroundSite where
  toFun := GroundSite.mk
  invFun := GroundSite.raw
  leftInv := by intro i; rfl
  rightInv := by intro g; cases g; rfl

/-- A pointwise kernel accepts only columns already aligned on one domain. -/
def pointwise {D : Type} {V : Type} (op : V → V → V)
    (left right : Field D V) : Field D V :=
  fun d => op (left d) (right d)

/--
error: Application type mismatch: The argument
  elevation
has type
  Field GroundSite Nat
but is expected to have type
  Field EntitySite Nat
in the application
  pointwise Nat.add health elevation
-/
#guard_msgs (error, drop info) in
#check fun (health : Field EntitySite Nat) (elevation : Field GroundSite Nat) =>
  pointwise Nat.add health elevation

/--
error: Application type mismatch: The argument
  health
has type
  Field EntitySite Nat
but is expected to have type
  Field GroundSite Nat
in the application
  Scatter.assign id ground health
-/
#guard_msgs (error, drop info) in
#check fun (ground : Field GroundSite Nat) (health : Field EntitySite Nat) =>
  Scatter.assign id ground health

/-- A registered semantic alignment is independent of either physical layout. -/
def declaredEntityToGround : Iso EntitySite GroundSite where
  toFun entity := ⟨entity.raw⟩
  invFun ground := ⟨ground.raw⟩
  leftInv entity := by cases entity; rfl
  rightInv ground := by cases ground; rfl

def pointwiseWithExplicitMap
    (health : Field EntitySite Nat) (elevation : Field GroundSite Nat) :
    Field EntitySite Nat :=
  pointwise Nat.add health (Field.reindex declaredEntityToGround elevation)

noncomputable def crossWriteWithExplicitMap
    (ground : Field GroundSite Nat) (health : Field EntitySite Nat) :
    Field GroundSite Nat :=
  Scatter.assign declaredEntityToGround ground health

theorem explicitCrossWriteDeterministic
    (ground : Field GroundSite Nat) (health : Field EntitySite Nat) :
    ∃ out, Scatter.Satisfies declaredEntityToGround ground health out ∧
      ∀ other, Scatter.Satisfies declaredEntityToGround ground health other → other = out :=
  Scatter.injective_scatter_deterministic declaredEntityToGround
    declaredEntityToGround.injective ground health

end Ano.NegativeWitness
