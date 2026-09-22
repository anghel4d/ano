import Ano.Field

namespace Ano

universe u v w x y

/-- The names and raw carriers of the fixed site domains in a world schema. -/
structure SiteSchema where
  Id : Type u
  Carrier : Id → Type v

/-- A site is nominally indexed by its schema and site name, even when raw carriers coincide. -/
structure Site (sites : SiteSchema.{u, v}) (id : sites.Id) where
  index : sites.Carrier id

/-- Registered fields have fixed nominal domains and schema-supplied validity predicates. -/
structure Schema where
  sites : SiteSchema.{u, v}
  FieldId : Type w
  fieldSite : FieldId → sites.Id
  Value : FieldId → Type x
  valid : (field : FieldId) →
    Field (Site sites (fieldSite field)) (Value field) → Prop
  globalValid : (entityCount : Nat) →
    ((field : FieldId) → Field (Site sites (fieldSite field)) (Value field)) → Prop
  spawnPreserves : ∀ entityCount count fields,
    (∀ field, valid field (fields field)) →
    globalValid entityCount fields →
    globalValid (entityCount + count) fields


/-- The semantic domain registered for one field. -/
abbrev FieldDomain (schema : Schema) (field : schema.FieldId) :=
  Site schema.sites (schema.fieldSite field)

/-- The heterogeneous family of all fixed fields registered by a schema. -/
abbrev RegisteredFields (schema : Schema) :=
  (field : schema.FieldId) → Field (FieldDomain schema field) (schema.Value field)

/-- Entity population is state-varying; registered fields remain indexed by the fixed schema. -/
structure World (schema : Schema) where
  entityCount : Nat
  fixed : RegisteredFields schema

/-- Entity keys are stable; liveness is evidence relative to a world state. -/
abbrev EntityKey := Nat

def Live {schema : Schema} (world : World schema) (key : EntityKey) : Prop :=
  key < world.entityCount

/-- Local field invariants and the schema-wide population invariant both hold. -/
def WellFormed {schema : Schema} (world : World schema) : Prop :=
  (∀ field, schema.valid field (world.fixed field)) ∧
    schema.globalValid world.entityCount world.fixed

/--
An ordinary update may write several registered fields. Its footprint conservatively contains
possibly changed cells; `writes` is a coarse field-level summary used by planners.
-/
structure OrdinaryUpdate (schema : Schema) where
  writes : schema.FieldId → Prop
  footprint : (field : schema.FieldId) → FieldDomain schema field → Prop
  footprint_writes : ∀ field site, footprint field site → writes field
  run : RegisteredFields schema → RegisteredFields schema
  preservesValid : ∀ fields,
    (∀ field, schema.valid field (fields field)) →
    ∀ field, schema.valid field (run fields field)
  preservesGlobal : ∀ entityCount fields,
    (∀ field, schema.valid field (fields field)) →
    schema.globalValid entityCount fields →
    schema.globalValid entityCount (run fields)
  frame : ∀ fields field site,
    ¬ footprint field site → run fields field site = fields field site

namespace OrdinaryUpdate

/-- Commit an ordinary update without changing the live entity population. -/
def apply {schema : Schema} (update : OrdinaryUpdate schema) (world : World schema) :
    World schema where
  entityCount := world.entityCount
  fixed := update.run world.fixed

@[simp] theorem apply_entityCount {schema : Schema} (update : OrdinaryUpdate schema)
    (world : World schema) :
    (update.apply world).entityCount = world.entityCount :=
  rfl

theorem apply_wellFormed {schema : Schema} (update : OrdinaryUpdate schema)
    {world : World schema} (hworld : WellFormed world) :
    WellFormed (update.apply world) := by
  constructor
  · exact update.preservesValid world.fixed hworld.1
  · exact update.preservesGlobal world.entityCount world.fixed
      hworld.1 hworld.2

theorem apply_frame {schema : Schema} (update : OrdinaryUpdate schema)
    (world : World schema) (field : schema.FieldId) (site : FieldDomain schema field)
    (outside : ¬ update.footprint field site) :
    (update.apply world).fixed field site = world.fixed field site :=
  update.frame world.fixed field site outside

/-- A field outside the write summary is extensionally unchanged. -/
theorem apply_unrelated {schema : Schema} (update : OrdinaryUpdate schema)
    (world : World schema) (field : schema.FieldId) (outside : ¬ update.writes field) :
    (update.apply world).fixed field = world.fixed field := by
  funext site
  exact update.apply_frame world field site fun inFootprint =>
    outside (update.footprint_writes field site inFootprint)

end OrdinaryUpdate

namespace World

/-- Spawn changes only the current entity population. -/
def spawn {schema : Schema} (world : World schema) (count : Nat) : World schema where
  entityCount := world.entityCount + count
  fixed := world.fixed

@[simp] theorem spawn_entityCount {schema : Schema} (world : World schema) (count : Nat) :
    (world.spawn count).entityCount = world.entityCount + count :=
  rfl

@[simp] theorem spawn_fixed {schema : Schema} (world : World schema) (count : Nat) :
    (world.spawn count).fixed = world.fixed :=
  rfl

@[simp] theorem spawn_field {schema : Schema} (world : World schema) (count : Nat)
    (field : schema.FieldId) :
    (world.spawn count).fixed field = world.fixed field :=
  rfl

theorem spawn_wellFormed {schema : Schema} {world : World schema}
    (hworld : WellFormed world) (count : Nat) :
    WellFormed (world.spawn count) := by
  constructor
  · exact hworld.1
  · exact schema.spawnPreserves world.entityCount count world.fixed
      hworld.1 hworld.2

end World

/-- A planner either refuses atomically or emits a certified ordinary/structural action. -/
inductive Plan (schema : Schema) (Output : Type y) where
  | refuse (reason : String)
  | ordinary (update : OrdinaryUpdate schema) (output : Output)
  | spawn (count : Nat) (output : Output)
  | ordinarySpawn (update : OrdinaryUpdate schema) (count : Nat) (output : Output)

abbrev Planner (schema : Schema) (Output : Type y) :=
  World schema → Plan schema Output

/-- Refusal retains the input world explicitly; success returns the committed world. -/
inductive Outcome (schema : Schema) (Output : Type y) where
  | refused (world : World schema) (reason : String)
  | ok (world : World schema) (output : Output)

/-- Planning observes pre-state; only an accepted certified action is committed. -/
def perform {schema : Schema} {Output : Type y} (planner : Planner schema Output)
    (world : World schema) : Outcome schema Output :=
  match planner world with
  | .refuse reason => .refused world reason
  | .ordinary update output => .ok (update.apply world) output
  | .spawn count output => .ok (world.spawn count) output
  | .ordinarySpawn update count output => .ok ((update.apply world).spawn count) output

/-- A direct planner refusal returns exactly the pre-state. -/
theorem perform_refusal_atomic {schema : Schema} {Output : Type y}
    (planner : Planner schema Output) (world : World schema) (reason : String)
    (refused : planner world = .refuse reason) :
    perform planner world = .refused world reason := by
  simp [perform, refused]

/-- Any refused outcome contains exactly the input world, never a partially changed one. -/
theorem perform_refused_world_eq {schema : Schema} {Output : Type y}
    (planner : Planner schema Output) (world before : World schema) (reason : String)
    (refused : perform planner world = .refused before reason) :
    before = world := by
  cases planEq : planner world with
  | refuse plannedReason =>
      simp [perform, planEq] at refused
      exact refused.1.symm
  | ordinary update output =>
      simp [perform, planEq] at refused
  | spawn count output =>
      simp [perform, planEq] at refused
  | ordinarySpawn update count output =>
      simp [perform, planEq] at refused

/-- Every accepted one-step plan preserves all registered field invariants. -/
theorem perform_ok_wellFormed {schema : Schema} {Output : Type y}
    (planner : Planner schema Output) {world after : World schema} {output : Output}
    (hworld : WellFormed world) (accepted : perform planner world = .ok after output) :
    WellFormed after := by
  cases planEq : planner world with
  | refuse reason =>
      simp [perform, planEq] at accepted
  | ordinary update plannedOutput =>
      simp [perform, planEq] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact update.apply_wellFormed hworld
  | spawn count plannedOutput =>
      simp [perform, planEq] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact World.spawn_wellFormed hworld count
  | ordinarySpawn update count plannedOutput =>
      simp [perform, planEq] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact World.spawn_wellFormed (update.apply_wellFormed hworld) count

/-- Extract only the successful successor; refusal has no successor state. -/
def next? {schema : Schema} {Output : Type y} (planner : Planner schema Output)
    (world : World schema) : Option (World schema) :=
  match perform planner world with
  | .refused _ _ => none
  | .ok after _ => some after

theorem next?_wellFormed {schema : Schema} {Output : Type y}
    (planner : Planner schema Output) {world after : World schema}
    (hworld : WellFormed world) (advanced : next? planner world = some after) :
    WellFormed after := by
  cases outcomeEq : perform planner world with
  | refused before reason =>
      simp [next?, outcomeEq] at advanced
  | ok successor output =>
      simp [next?, outcomeEq] at advanced
      subst after
      exact perform_ok_wellFormed planner hworld outcomeEq

/-- Run the same tick planner a finite number of times, stopping at the first refusal. -/
def ticks {schema : Schema} {Output : Type y} (planner : Planner schema Output) :
    Nat → World schema → Option (World schema)
  | 0, world => some world
  | steps + 1, world =>
      match next? planner world with
      | none => none
      | some after => ticks planner steps after

/-- Any finite sequence of successful ticks preserves the registered schema invariants. -/
theorem ticks_wellFormed {schema : Schema} {Output : Type y}
    (planner : Planner schema Output) (steps : Nat) {world after : World schema}
    (hworld : WellFormed world) (ran : ticks planner steps world = some after) :
    WellFormed after := by
  induction steps generalizing world with
  | zero =>
      simp [ticks] at ran
      subst after
      exact hworld
  | succ steps inductionHypothesis =>
      cases advancedEq : next? planner world with
      | none =>
          simp [ticks, advancedEq] at ran
      | some successor =>
          have hsuccessor : WellFormed successor :=
            next?_wellFormed planner hworld advancedEq
          have hrest : ticks planner steps successor = some after := by
            simpa [ticks, advancedEq] using ran
          exact inductionHypothesis hsuccessor hrest

end Ano
