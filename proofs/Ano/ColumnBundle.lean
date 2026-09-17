import Ano.Lineage
import Ano.SpatialWorld

namespace Ano.ColumnBundle

universe u v w x q r s t

/--
A total point view over one query domain, certified by live lineage and the partial component mask.
The stored equation is what turns `Option Point` into `Point`; row count alone cannot do so.
-/
structure PresentPositionRows
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position) (J : Type s) where
  lineage : SpatialWorld.LiveLineage world J
  values : Field J (Point frames frame)
  stored : ∀ row, world.position (lineage row) = some (values row)

namespace PresentPositionRows

@[simp] theorem gathered
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (rows : PresentPositionRows world J) (row : J) :
    world.position (rows.lineage row) = some (rows.values row) :=
  rows.stored row

end PresentPositionRows

/--
One input is an actual fixed schema field gathered through sealed lineage, a registered weighted
interpolation of that field, or the partial live Position column with presence evidence.
There is no constructor from a layout, shape, cardinality equality, raw index map, or raw span.
-/
inductive RegisteredInput
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position) (J : Type s) :
    Type (max x v) → Type (max (max (max (max u v) w) (q + 1)) (max x s)) where
  | fixedField (field : schema.FieldId)
      (lineage : Lineage spatial J (FieldDomain schema field)) :
      RegisteredInput world J (ULift.{v} (schema.Value field))
  | livePosition (rows : PresentPositionRows world J) :
      RegisteredInput world J (ULift.{x} (Point frames frame))
  | interpolatedField (field : schema.FieldId)
      {K : Type q} {weights : WeightLaw K}
      {authority : SpatialInterpolationRegistry.{u, v, w, x, u, v, w, q, q, q}
        spatial K weights}
      {token : authority.Token (schema.fieldSite field) frame}
      {positions : Field J (Point frames frame)}
      (rows : RegisteredInterpolationRows spatial authority token positions)
      (values : WeightedValueLaw weights (schema.Value field)) :
      RegisteredInput world J (ULift.{v} (schema.Value field))

namespace RegisteredInput

/-- Every admitted input is gathered onto the one current query domain `J`. -/
def aligned
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    {V : Type (max x v)} (input : RegisteredInput world J V) : Field J V :=
  match input with
  | .fixedField field lineage =>
      fun row => ULift.up (world.base.fixed field (lineage row))
  | .livePosition rows => fun row => ULift.up (rows.values row)
  | .interpolatedField field rows values =>
      fun row => ULift.up
        (Lineage.interpolationSample rows values (world.base.fixed field) row)

@[simp] theorem aligned_fixedField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (field : schema.FieldId)
    (lineage : Lineage spatial J (FieldDomain schema field)) (row : J) :
    (RegisteredInput.fixedField field lineage :
      RegisteredInput world J (ULift.{v} (schema.Value field))).aligned row =
        ULift.up (world.base.fixed field (lineage row)) :=
  rfl

@[simp] theorem aligned_livePosition
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (rows : PresentPositionRows world J) (row : J) :
    (RegisteredInput.livePosition rows :
      RegisteredInput world J (ULift.{x} (Point frames frame))).aligned row = ULift.up (rows.values row) :=
  rfl

theorem aligned_livePosition_stored
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (rows : PresentPositionRows world J) (row : J) :
    world.position (rows.lineage row) =
      some ((RegisteredInput.livePosition rows :
        RegisteredInput world J (ULift.{x} (Point frames frame))).aligned row).down :=
  rows.stored row

/-- A successful registered locator is the singleton-gather case of a bundle input. -/
def locatedField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (field : schema.FieldId)
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {positions : Field J (Point frames frame)}
    (rows : LocatedRows spatial situated positions) :
    RegisteredInput world J (ULift.{v} (schema.Value field)) :=
  .fixedField field (Lineage.located rows)

@[simp] theorem aligned_locatedField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (field : schema.FieldId)
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {positions : Field J (Point frames frame)}
    (rows : LocatedRows spatial situated positions) (row : J) :
    (locatedField (world := world) field rows).aligned row =
      ULift.up (world.base.fixed field (rows.destination row)) :=
  rfl

@[simp] theorem aligned_interpolatedField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s}
    (field : schema.FieldId)
    {K : Type q} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry.{u, v, w, x, u, v, w, q, q, q}
      spatial K weights}
    {token : authority.Token (schema.fieldSite field) frame}
    {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows spatial authority token positions)
    (values : WeightedValueLaw weights (schema.Value field)) (row : J) :
    (RegisteredInput.interpolatedField (world := world) field rows values).aligned row =
      ULift.up
        (Lineage.interpolationSample rows values (world.base.fixed field) row) :=
  rfl

end RegisteredInput

/-- A genuinely heterogeneous family of registered inputs sharing exactly one query domain. -/
structure Bundle
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position)
    (J : Type s) (n : Nat) where
  Value : Fin n → Type (max x v)
  input : (column : Fin n) → RegisteredInput world J (Value column)

namespace Bundle

/-- The dependent row record consumed by an n-ary pointwise evaluator. -/
abbrev Row
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n) :=
  (column : Fin n) → bundle.Value column

/-- Read one dependent tuple; every component was independently gathered onto the same row. -/
def row
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n) (queryRow : J) :
    bundle.Row :=
  fun column => (bundle.input column).aligned queryRow

@[simp] theorem row_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n)
    (queryRow : J) (column : Fin n) :
    bundle.row queryRow column = (bundle.input column).aligned queryRow :=
  rfl

/-- Pointwise n-ary mapping preserves the common query domain by construction. -/
def map
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n)
    {Output : Type t} (evaluate : bundle.Row → Output) : Field J Output :=
  fun queryRow => evaluate (bundle.row queryRow)

@[simp] theorem map_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n)
    {Output : Type t} (evaluate : bundle.Row → Output) (queryRow : J) :
    bundle.map evaluate queryRow = evaluate (bundle.row queryRow) :=
  rfl

end Bundle

/--
A typed n-ary result eligible for the registered live `Position<frame>` component. The arbitrary
evaluator is generic typed metalanguage; an accepted Ano plan still needs Steel to certify that its
carrier operations came from the registry rather than treating this public function as authority.
-/
structure PositionMapping
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n) where
  evaluate : bundle.Row → Point frames frame

namespace PositionMapping

def values
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (mapping : PositionMapping bundle) : Field J (Point frames frame) :=
  bundle.map mapping.evaluate

@[simp] theorem values_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (mapping : PositionMapping bundle) (row : J) :
    mapping.values row = mapping.evaluate (bundle.row row) :=
  rfl

end PositionMapping

/-- Plain entity-position assignment requires sealed live lineage and injective destinations. -/
structure EntityPositionEffect
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} (bundle : Bundle world J n) where
  destination : SpatialWorld.LiveLineage world J
  destination_injective : Function.Injective destination
  mapping : PositionMapping bundle

namespace EntityPositionEffect

/-- Commit only the registered partial Position component; population and fixed fields do not move. -/
noncomputable def apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) :
    SpatialWorld schema frames spatial entities position where
  base := world.base
  position := Scatter.assign effect.destination world.position
    (fun row => some (effect.mapping.values row))

@[simp] theorem apply_base
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) :
    effect.apply.base = world.base :=
  rfl

@[simp] theorem apply_entityCount
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) :
    effect.apply.base.entityCount = world.base.entityCount :=
  rfl

@[simp] theorem apply_fixed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) :
    effect.apply.base.fixed = world.base.fixed :=
  rfl

theorem apply_fixed_field
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) (field : schema.FieldId) :
    effect.apply.base.fixed field = world.base.fixed field :=
  rfl

theorem apply_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle)
    (wellFormed : SpatialWorld.WellFormed world) :
    SpatialWorld.WellFormed effect.apply :=
  wellFormed

@[simp] theorem apply_hit
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) (row : J) :
    effect.apply.position (effect.destination row) =
      some (effect.mapping.values row) :=
  Scatter.assign_hit effect.destination effect.destination_injective world.position
    (fun queryRow => some (effect.mapping.values queryRow)) row

/--
The complete registered path reads every heterogeneous input on one query row, evaluates one
frame-correct point, and scatters that point to the certified live entity destination.
-/
theorem apply_hit_registered_inputs
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) (row : J) :
    effect.apply.position (effect.destination row) =
      some (effect.mapping.evaluate
        (fun column => (bundle.input column).aligned row)) := by
  rw [effect.apply_hit]
  rfl

theorem apply_miss
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (effect : EntityPositionEffect bundle) (key : Fin world.base.entityCount)
    (miss : ¬ ∃ row, effect.destination row = key) :
    effect.apply.position key = world.position key :=
  Scatter.assign_miss effect.destination world.position
    (fun queryRow => some (effect.mapping.values queryRow)) key miss

end EntityPositionEffect

/--
Point results become lattice-cell destinations only through one exact registry `SituatedToken`
and a proof that every row located. The destination type is the target field's nominal habitat.
-/
structure LatticeDestinations
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    (field : schema.FieldId)
    (situated : spatial.SituatedToken (schema.fieldSite field) frame)
    (points : PositionMapping bundle) where
  rows : LocatedRows spatial situated points.values

namespace LatticeDestinations

/-- The certified cell column retains sealed point-to-habitat lineage. -/
def lineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) :
    Lineage spatial J (FieldDomain schema field) :=
  Lineage.located destinations.rows

def cells
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) :
    Field J (FieldDomain schema field) :=
  destinations.lineage

@[simp] theorem cells_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) (row : J) :
    destinations.cells row = destinations.rows.destination row :=
  rfl

/-- Gather the target fixed field through the same sealed located lineage. -/
def sample
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) :
    Field J (schema.Value field) :=
  Lineage.gather destinations.lineage (world.base.fixed field)

@[simp] theorem sample_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) (row : J) :
    destinations.sample row = world.base.fixed field (destinations.cells row) :=
  rfl

theorem cells_accepted
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) (row : J) :
    (spatial.situated situated).locator.accepts
      (points.values row) (destinations.cells row) :=
  destinations.rows.destination_accepted row

end LatticeDestinations

/-- A collision-free plain assignment into one fixed lattice field. -/
structure LatticeAssignment
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    (destinations : LatticeDestinations field situated points) where
  destination_injective : Function.Injective destinations.cells
  evaluate : bundle.Row → schema.Value field

namespace LatticeAssignment

def values
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    {destinations : LatticeDestinations field situated points}
    (assignment : LatticeAssignment destinations) : Field J (schema.Value field) :=
  bundle.map assignment.evaluate

/--
This constructs the target field value. Promoting it to a world action still requires the
schema's `valid` and `globalValid` preservation certificates; this module does not claim them.
-/
noncomputable def applyField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    {destinations : LatticeDestinations field situated points}
    (assignment : LatticeAssignment destinations)
    (base : Field (FieldDomain schema field) (schema.Value field)) :
    Field (FieldDomain schema field) (schema.Value field) :=
  Scatter.assign destinations.cells base assignment.values

@[simp] theorem applyField_hit
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    {destinations : LatticeDestinations field situated points}
    (assignment : LatticeAssignment destinations)
    (base : Field (FieldDomain schema field) (schema.Value field)) (row : J) :
    assignment.applyField base (destinations.cells row) = assignment.values row :=
  Scatter.assign_hit destinations.cells assignment.destination_injective
    base assignment.values row

theorem applyField_miss
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {J : Type s} {n : Nat} {bundle : Bundle world J n}
    {field : schema.FieldId}
    {situated : spatial.SituatedToken (schema.fieldSite field) frame}
    {points : PositionMapping bundle}
    {destinations : LatticeDestinations field situated points}
    (assignment : LatticeAssignment destinations)
    (base : Field (FieldDomain schema field) (schema.Value field))
    (cell : FieldDomain schema field)
    (miss : ¬ ∃ row, destinations.cells row = cell) :
    assignment.applyField base cell = base cell :=
  Scatter.assign_miss destinations.cells base assignment.values cell miss

end LatticeAssignment

end Ano.ColumnBundle
