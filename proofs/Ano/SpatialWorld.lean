import Ano.SpatialSpawn

namespace Ano

universe u v w x y z p q r s

/-- A partial column is a semantic value-or-absence family over its typed row domain. -/
abbrev PartialColumn (D : Type u) (V : Type v) := Field D (Option V)

/-- `Position<frame>` varies over the current live entity keys, never over a fixed habitat. -/
abbrev PositionColumn (frames : FrameSchema.{u, v, w}) (frame : frames.Id)
    (entityCount : Nat) :=
  PartialColumn (Fin entityCount) (Point frames frame)

namespace PositionColumn

/-- Preserve the old mask/value rows and make every fresh row present with its supplied point. -/
def extend {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {oldCount added : Nat}
    (old : PositionColumn frames frame oldCount)
    (fresh : Field (Fin added) (Point frames frame)) :
    PositionColumn frames frame (oldCount + added) :=
  Fin.addCases old (fun row => some (fresh row))

@[simp] theorem extend_old {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {oldCount added : Nat}
    (old : PositionColumn frames frame oldCount)
    (fresh : Field (Fin added) (Point frames frame))
    (key : Fin oldCount) :
    extend old fresh (Allocation.oldKey (added := added) key) = old key := by
  simp [extend, Allocation.oldKey]

@[simp] theorem extend_fresh {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {oldCount added : Nat}
    (old : PositionColumn frames frame oldCount)
    (fresh : Field (Fin added) (Point frames frame))
    (row : Fin added) :
    extend old fresh (Allocation.freshKey oldCount row) = some (fresh row) := by
  simp [extend, Allocation.freshKey]

end PositionColumn

/--
The existing `SpatialRegistry.PositionToken` is indexed by a fixed `FieldId`, so it cannot
authorize a partial component over a state-varying live population. This companion registry
family is the missing nominal capability; its values are supplied by the registry, not inferred
from a component name, item shape, frame dimension, or row count.
-/
structure EntitySpatialRegistry
    (schema : Schema.{u, v, w, x}) (frames : FrameSchema.{u, v, w})
    (_spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames) where
  PositionToken : frames.Id → Type r
  PlayerResolverToken : Type r

/-- A richer world pairs the fixed-field world with one registered partial live position column. -/
structure SpatialWorld
    (schema : Schema.{u, v, w, x}) (frames : FrameSchema.{u, v, w})
    (spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames)
    (entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial)
    {frame : frames.Id} (_position : entities.PositionToken frame) where
  base : World schema
  position : PositionColumn frames frame base.entityCount

namespace SpatialWorld

/-- Typed entity lineage is indexed by the exact destination world value. -/
structure LiveLineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position) (J : Type s) where
  private mk ::
  toKey : J → Fin world.base.entityCount

namespace LiveLineage

instance
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position} {J : Type s} :
    CoeFun (LiveLineage world J) (fun _ => J → Fin world.base.entityCount) :=
  ⟨LiveLineage.toKey⟩

/-- Identity is the only ambient live-key lineage available without changing the query view. -/
def refl
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position) :
    LiveLineage world (Fin world.base.entityCount) where
  toKey := id

/-- A frozen selection of live keys retains its subtype inclusion as live lineage. -/
def selection
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position)
    (selected : Selection (Fin world.base.entityCount)) :
    LiveLineage world selected.Row where
  toKey := selected.inclusion

end LiveLineage
/--
A registered singleton player resolution over the exact pre-world. Its private constructor prevents
a generic Entity := Unit source from masquerading as a live entity anchor in the richer path.
-/
structure LivePlayerAnchor
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position)
    (_resolver : entities.PlayerResolverToken) where
  private mk ::
  selected : Selection (Fin world.base.entityCount)
  player : selected.Row
  unique : ∀ other : selected.Row, other = player
  anchor : Point frames frame
  stored : world.position player.val = some anchor

namespace LivePlayerAnchor

/-- The sole factory requires the registry-owned resolver token and a proved singleton live view. -/
def registered
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    (resolver : entities.PlayerResolverToken)
    (selected : Selection (Fin world.base.entityCount))
    (player : selected.Row) (unique : ∀ other : selected.Row, other = player)
    (anchor : Point frames frame)
    (stored : world.position player.val = some anchor) :
    LivePlayerAnchor world resolver where
  selected := selected
  player := player
  unique := unique
  anchor := anchor
  stored := stored

/-- The selected row retains sealed lineage into the exact pre-world live population. -/
def lineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    LiveLineage world player.selected.Row :=
  LiveLineage.selection world player.selected

/-- The exact live key selected by the registered resolver. -/
def key
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) : Fin world.base.entityCount :=
  player.lineage player.player

@[simp] theorem key_eq
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    player.key = player.player.val :=
  rfl

theorem position_at_key
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    world.position player.key = some player.anchor :=
  player.stored

/-- Every copy row gathers the same selected live source key. -/
def copyKey
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    SpatialSpawn.CheeseCopies → Fin world.base.entityCount :=
  fun _ => player.key

/-- Copy gathering recovers the certified anchor from the partial Position component. -/
theorem copy_position_gather
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) (copy : SpatialSpawn.CheeseCopies) :
    world.position (player.copyKey copy) = some player.anchor :=
  player.position_at_key

/-- A total view is used only to satisfy the old pattern carrier; the selected row is exact. -/
def totalPosition
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    Field (Fin world.base.entityCount) (Point frames frame) :=
  fun key =>
    match world.position key with
    | some point => point
    | none => player.anchor

@[simp] theorem totalPosition_player
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver) :
    player.totalPosition player.key = player.anchor := by
  rw [totalPosition, player.position_at_key]

/-- Build the old command pattern only from this live anchor and a registered offset map. -/
def toPattern
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (embedding : spatial.OffsetMapToken plane frame)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    SpatialSpawn.PlayerPattern (Fin world.base.entityCount) frames plane frame :=
  SpatialSpawn.PlayerPattern.registered spatial embedding player.selected.keep player.key
    player.player.property
    (fun other selected =>
      congrArg Subtype.val (player.unique ⟨other, selected⟩))
    player.totalPosition offset offset_injective

@[simp] theorem toPattern_player_position
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (embedding : spatial.OffsetMapToken plane frame)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    (player.toPattern embedding offset offset_injective).position
        (player.toPattern embedding offset offset_injective).player =
      player.anchor := by
  change player.totalPosition player.key = player.anchor
  exact player.totalPosition_player

/-- Every seed is anchored at the point actually read from the exact pre-world Position row. -/
theorem toPattern_seed_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (embedding : spatial.OffsetMapToken plane frame)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset)
    (copy : SpatialSpawn.CheeseCopies) :
    (player.toPattern embedding offset offset_injective).seeds copy =
      (spatial.offsetMap embedding).place player.anchor (offset copy) := by
  change (spatial.offsetMap embedding).place
    (player.totalPosition player.key) (offset copy) = _
  rw [player.totalPosition_player]

/-- Retain the registered scalar-linear witness on the exact live-anchor pattern. -/
def toLinearPattern
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {plane frame : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {frameModule : ModuleLaw scalars (affine.space frame).vectors}
    {embedding : spatial.OffsetMapToken plane frame}
    {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (linear : RegisteredLinearAffineEmbedding spatial affine scalars
      planeModule frameModule embedding)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    SpatialSpawn.LinearPlayerPattern spatial affine scalars planeModule frameModule
      embedding (Fin world.base.entityCount) :=
  SpatialSpawn.PlayerPattern.registeredLinear linear player.selected.keep player.key
    player.player.property
    (fun other selected =>
      congrArg Subtype.val (player.unique ⟨other, selected⟩))
    player.totalPosition offset offset_injective

/-- Forgetting the linear witness recovers exactly the live pattern used by execution. -/
@[simp] theorem toLinearPattern_pattern
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {plane frame : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {frameModule : ModuleLaw scalars (affine.space frame).vectors}
    {embedding : spatial.OffsetMapToken plane frame}
    {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (linear : RegisteredLinearAffineEmbedding spatial affine scalars
      planeModule frameModule embedding)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    (player.toLinearPattern linear offset offset_injective).pattern =
      player.toPattern embedding offset offset_injective :=
  rfl

/-- The linear refinement keeps every seed anchored at the certified live Position value. -/
theorem toLinearPattern_seed_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {plane frame : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {frameModule : ModuleLaw scalars (affine.space frame).vectors}
    {embedding : spatial.OffsetMapToken plane frame}
    {position : entities.PositionToken frame}
    {world : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    (player : LivePlayerAnchor world resolver)
    (linear : RegisteredLinearAffineEmbedding spatial affine scalars
      planeModule frameModule embedding)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset)
    (copy : SpatialSpawn.CheeseCopies) :
    (player.toLinearPattern linear offset offset_injective).seeds copy =
      (spatial.offsetMap embedding).place player.anchor (offset copy) := by
  change
    (player.toLinearPattern linear offset offset_injective).pattern.seeds copy = _
  rw [player.toLinearPattern_pattern linear offset offset_injective]
  exact player.toPattern_seed_apply embedding offset offset_injective copy

end LivePlayerAnchor


/-- The exact structural commit extends live positions and leaves every fixed field untouched. -/
def commitCheese
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    SpatialWorld schema frames spatial entities position where
  base := before.base.spawn 51
  position := PositionColumn.extend before.position payload.positions

@[simp] theorem commitCheese_entityCount
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    (commitCheese before payload).base.entityCount = before.base.entityCount + 51 :=
  rfl

@[simp] theorem commitCheese_fixed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    (commitCheese before payload).base.fixed = before.base.fixed :=
  rfl

theorem commitCheese_fixed_field
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) (field : schema.FieldId) :
    (commitCheese before payload).base.fixed field = before.base.fixed field :=
  rfl

@[simp] theorem commitCheese_old_position
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) (old : Fin before.base.entityCount) :
    (commitCheese before payload).position
        (Allocation.oldKey (added := 51) old) = before.position old := by
  simp [commitCheese]

@[simp] theorem commitCheese_fresh_position
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) (copy : SpatialSpawn.CheeseCopies) :
    (commitCheese before payload).position
        (SpatialSpawn.freshKey before.base.entityCount copy) =
      some (payload.positions copy) := by
  simp [commitCheese, SpatialSpawn.freshKey]

theorem commitCheese_fresh_present_supported
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) (copy : SpatialSpawn.CheeseCopies) :
    (commitCheese before payload).position
        (SpatialSpawn.freshKey before.base.entityCount copy) =
          some (payload.positions copy) ∧
      payload.batch.resting.supported (payload.batch.resolved.hits copy)
        (payload.positions copy) :=
  ⟨commitCheese_fresh_position before payload copy,
    payload.every_position_is_supported copy⟩

/-- Gathering the enlarged component through the old-key embedding recovers the old component. -/
theorem commitCheese_old_positions_ext
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    Field.reindex (Allocation.oldKey (added := 51))
        (commitCheese before payload).position =
      before.position := by
  funext old
  exact commitCheese_old_position before payload old

/-- Gathering through the fresh allocation recovers exactly the complete supported payload. -/
theorem commitCheese_fresh_positions_ext
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    Field.reindex (SpatialSpawn.freshKey before.base.entityCount)
        (commitCheese before payload).position =
      fun copy => some (payload.positions copy) := by
  funext copy
  exact commitCheese_fresh_position before payload copy

/--
Every post-commit live key is either one preserved old key or one of the fifty-one allocated
copy keys, with exactly the corresponding old or payload position. There is no third segment.
-/
theorem commitCheese_position_exhaustive
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support)
    (key : Fin (commitCheese before payload).base.entityCount) :
    (∃ old : Fin before.base.entityCount,
        key = Allocation.oldKey (added := 51) old ∧
          (commitCheese before payload).position key = before.position old) ∨
      (∃ copy : SpatialSpawn.CheeseCopies,
        key = SpatialSpawn.freshKey before.base.entityCount copy ∧
          (commitCheese before payload).position key =
            some (payload.positions copy)) := by
  exact Fin.addCases
    (fun old => Or.inl ⟨old, rfl, commitCheese_old_position before payload old⟩)
    (fun copy =>
      Or.inr ⟨copy, rfl, commitCheese_fresh_position before payload copy⟩)
    key

/-- Old keys retain explicit lineage into the exact enlarged world. -/
def oldLineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    LiveLineage (commitCheese before payload) (Fin before.base.entityCount) where
  toKey := Allocation.oldKey

/-- Copy rows receive explicit fresh-entity lineage only through the exact allocation map. -/
def freshLineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    LiveLineage (commitCheese before payload) SpatialSpawn.CheeseCopies where
  toKey := SpatialSpawn.freshKey before.base.entityCount

theorem freshLineage_injective
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    (before : SpatialWorld schema frames spatial entities position)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    Function.Injective (freshLineage before payload) :=
  SpatialSpawn.freshKey_injective before.base.entityCount

/-- The richer invariant retains the base schema law; live-key bounds and point frame are typed. -/
def WellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {frame : frames.Id} {position : entities.PositionToken frame}
    (world : SpatialWorld schema frames spatial entities position) : Prop :=
  Ano.WellFormed world.base

theorem commitCheese_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    {support : SnapshotSupportProjector (World schema) Surface frames frame Score}
    {before : SpatialWorld schema frames spatial entities position}
    (wellFormed : WellFormed before)
    (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      support) :
    WellFormed (commitCheese before payload) :=
  World.spawn_wellFormed wellFormed 51

/-- Generic certified-payload executor. Entity remains arbitrary, so this is not the accepted live-anchor path. -/
inductive CheeseOutcome
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score) where
  | refused (world : SpatialWorld schema frames spatial entities position) (reason : String)
  | ok (world : SpatialWorld schema frames spatial entities position)
      (payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
        command.support)

def performCheese
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) (before : SpatialWorld schema frames spatial entities position) :
    CheeseOutcome (spatial := spatial) (entities := entities)
      (position := position) command :=
  match command.prepare before.base with
  | none => .refused before reason
  | some payload => .ok (commitCheese before payload) payload

theorem performCheese_refusal_atomic
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) (before : SpatialWorld schema frames spatial entities position)
    (rejected : command.prepare before.base = none) :
    performCheese command reason before = .refused before reason := by
  simp [performCheese, rejected]

theorem performCheese_miss_refuses_atomically
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) (before : SpatialWorld schema frames spatial entities position)
    (miss : ∃ copy : SpatialSpawn.CheeseCopies,
      (command.support.freeze before.base).locator.locate
        ((command.pattern before.base).seeds copy) = none) :
    performCheese command reason before = .refused before reason := by
  exact performCheese_refusal_atomic command reason before
    ((command.prepare_none_iff before.base).2 miss)

theorem performCheese_success_world
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) {before after : SpatialWorld schema frames spatial entities position}
    {payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      command.support}
    (accepted : performCheese command reason before = .ok after payload) :
    after = commitCheese before payload := by
  cases prepared : command.prepare before.base with
  | none => simp [performCheese, prepared] at accepted
  | some planned =>
      simp [performCheese, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      rfl

theorem performCheese_success_payload_snapshot
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) {before after : SpatialWorld schema frames spatial entities position}
    {payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      command.support}
    (accepted : performCheese command reason before = .ok after payload) :
    payload.snapshot = before.base := by
  cases prepared : command.prepare before.base with
  | none => simp [performCheese, prepared] at accepted
  | some planned =>
      have sameSnapshot := command.prepared_snapshot_eq before.base prepared
      simp [performCheese, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact sameSnapshot

theorem performCheese_success_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) {before after : SpatialWorld schema frames spatial entities position}
    {payload : SpatialSpawn.FrozenPreparedBatch schema Entity Surface frames plane frame Score
      command.support}
    (wellFormed : WellFormed before)
    (accepted : performCheese command reason before = .ok after payload) :
    WellFormed after := by
  rw [performCheese_success_world command reason accepted]
  exact commitCheese_wellFormed wellFormed payload

/--
The accepted one-shot request closes the old generic Entity gap. Its command source is exactly the
live-key type of before, and its pattern at before.base is exactly the registered live anchor pattern.
-/
structure LiveCheeseRequest
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    (before : SpatialWorld schema frames spatial entities position)
    (_resolver : entities.PlayerResolverToken)
    (Surface : Type u) (Score : Type v) where
  private mk ::
  anchor : LivePlayerAnchor before _resolver
  embedding : spatial.OffsetMapToken plane frame
  offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane)
  offset_injective : Function.Injective offset
  command : SpatialSpawn.CheeseCommand schema (Fin before.base.entityCount) Surface
    frames plane frame Score
  pattern_exact :
    command.pattern before.base =
      anchor.toPattern embedding offset offset_injective

namespace LiveCheeseRequest

/-- The sole request factory requires the live anchor, registered embedding, and exact pattern law. -/
def registered
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before : SpatialWorld schema frames spatial entities position}
    (resolver : entities.PlayerResolverToken)
    {Surface : Type u} {Score : Type v}
    (anchor : LivePlayerAnchor before resolver)
    (embedding : spatial.OffsetMapToken plane frame)
    (offset : Field SpatialSpawn.CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset)
    (command : SpatialSpawn.CheeseCommand schema (Fin before.base.entityCount) Surface
      frames plane frame Score)
    (pattern_exact :
      command.pattern before.base =
        anchor.toPattern embedding offset offset_injective) :
    LiveCheeseRequest (plane := plane) before resolver Surface Score where
  anchor := anchor
  embedding := embedding
  offset := offset
  offset_injective := offset_injective
  command := command
  pattern_exact := pattern_exact

/-- This is the richer accepted path: preparation and support read before.base, then commit before. -/
def performLiveCheese
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String) :
    CheeseOutcome (spatial := spatial) (entities := entities)
      (plane := plane) (position := position) request.command :=
  performCheese request.command reason before

theorem performLiveCheese_refusal_atomic
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String) (rejected : request.command.prepare before.base = none) :
    request.performLiveCheese reason = .refused before reason := by
  exact performCheese_refusal_atomic request.command reason before rejected

theorem performLiveCheese_miss_refuses_atomically
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    (miss : ∃ copy : SpatialSpawn.CheeseCopies,
      (request.command.support.freeze before.base).locator.locate
        ((request.command.pattern before.base).seeds copy) = none) :
    request.performLiveCheese reason = .refused before reason :=
  performCheese_miss_refuses_atomically request.command reason before miss

theorem performLiveCheese_success_world
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload) :
    after = commitCheese before payload :=
  performCheese_success_world request.command reason accepted

theorem performLiveCheese_success_payload_snapshot
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload) :
    payload.snapshot = before.base :=
  performCheese_success_payload_snapshot request.command reason accepted

/-- The accepted payload pattern is exactly the registered live-anchor pattern for before. -/
theorem performLiveCheese_success_pattern
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload) :
    payload.batch.pattern =
      request.anchor.toPattern request.embedding request.offset request.offset_injective := by
  cases prepared : request.command.prepare before.base with
  | none => simp [performLiveCheese, performCheese, prepared] at accepted
  | some planned =>
      have exactPattern := request.command.prepared_pattern_eq before.base prepared
      simp [performLiveCheese, performCheese, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact exactPattern.trans request.pattern_exact

/-- The successful payload retains any registered linear witness for the request's exact token. -/
theorem performLiveCheese_success_linear_pattern
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {plane frame : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {frameModule : ModuleLaw scalars (affine.space frame).vectors}
    {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (linear : RegisteredLinearAffineEmbedding spatial affine scalars
      planeModule frameModule request.embedding)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload) :
    payload.batch.pattern =
      (request.anchor.toLinearPattern linear request.offset
        request.offset_injective).pattern :=
  (request.performLiveCheese_success_pattern reason accepted).trans
    (request.anchor.toLinearPattern_pattern linear request.offset
      request.offset_injective).symm

/-- Every accepted seed uses the point read from the registered live Position row as its anchor. -/
theorem performLiveCheese_success_seed_anchor
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload)
    (copy : SpatialSpawn.CheeseCopies) :
    payload.batch.pattern.seeds copy =
      (spatial.offsetMap request.embedding).place request.anchor.anchor
        (request.offset copy) := by
  rw [request.performLiveCheese_success_pattern reason accepted]
  exact request.anchor.toPattern_seed_apply request.embedding request.offset
    request.offset_injective copy

/-- Success is the exact +51 commit, with old/fresh Position segments and fixed fields unchanged. -/
theorem performLiveCheese_success_exact_commit
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload) :
    after = commitCheese before payload ∧
      (commitCheese before payload).base.entityCount = before.base.entityCount + 51 ∧
      (commitCheese before payload).base.fixed = before.base.fixed ∧
      Field.reindex (Allocation.oldKey (added := 51))
          (commitCheese before payload).position = before.position ∧
      Field.reindex (SpatialSpawn.freshKey before.base.entityCount)
          (commitCheese before payload).position =
        fun copy => some (payload.positions copy) :=
  ⟨request.performLiveCheese_success_world reason accepted,
    commitCheese_entityCount before payload,
    commitCheese_fixed before payload,
    commitCheese_old_positions_ext before payload,
    commitCheese_fresh_positions_ext before payload⟩

/-- Every fresh live Position is present and certified against the same frozen support snapshot. -/
theorem performLiveCheese_success_fresh_present_supported
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (accepted : request.performLiveCheese reason = .ok after payload)
    (copy : SpatialSpawn.CheeseCopies) :
    ∃ key : Fin after.base.entityCount,
      key.val = (SpatialSpawn.freshKey before.base.entityCount copy).val ∧
        after.position key = some (payload.positions copy) ∧
          payload.batch.resting.supported (payload.batch.resolved.hits copy)
            (payload.positions copy) := by
  have afterEq : after = commitCheese before payload :=
    request.performLiveCheese_success_world reason accepted
  subst after
  refine ⟨SpatialSpawn.freshKey before.base.entityCount copy, rfl, ?_⟩
  exact commitCheese_fresh_present_supported before payload copy

theorem performLiveCheese_success_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {before after : SpatialWorld schema frames spatial entities position}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (request : LiveCheeseRequest (plane := plane) before resolver Surface Score)
    (reason : String)
    {payload : SpatialSpawn.FrozenPreparedBatch schema
      (Fin before.base.entityCount) Surface frames plane frame Score request.command.support}
    (wellFormed : WellFormed before)
    (accepted : request.performLiveCheese reason = .ok after payload) :
    WellFormed after :=
  performCheese_success_wellFormed request.command reason wellFormed accepted

end LiveCheeseRequest

/--
A live resolver must rebuild a sealed request against the world supplied at that tick. Its dependent
result prevents retaining the first world's Fin carrier, anchor row, or Position evidence.
-/
structure LiveCheeseResolver
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    (resolver : entities.PlayerResolverToken)
    (Surface : Type u) (Score : Type v) where
  request : (world : SpatialWorld schema frames spatial entities position) →
    Option (LiveCheeseRequest (plane := plane) world resolver Surface Score)

namespace LiveCheeseResolver

/-- One live step resolves the registered anchor from the current world and then executes it. -/
def nextLiveCheese?
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (planner : LiveCheeseResolver (plane := plane) (frame := frame) (position := position) resolver Surface Score)
    (reason : String)
    (before : SpatialWorld schema frames spatial entities position) :
    Option (SpatialWorld schema frames spatial entities position) :=
  match planner.request before with
  | none => none
  | some request =>
      match request.performLiveCheese reason with
      | .refused _ _ => none
      | .ok after _ => some after

theorem nextLiveCheese?_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (planner : LiveCheeseResolver (plane := plane) (frame := frame) (position := position) resolver Surface Score)
    (reason : String)
    {before after : SpatialWorld schema frames spatial entities position}
    (wellFormed : WellFormed before)
    (advanced : planner.nextLiveCheese? reason before = some after) :
    WellFormed after := by
  cases requestEq : planner.request before with
  | none =>
      simp [nextLiveCheese?, requestEq] at advanced
  | some request =>
      cases outcomeEq : request.performLiveCheese reason with
      | refused snapshot refusal =>
          simp [nextLiveCheese?, requestEq, outcomeEq] at advanced
      | ok successor payload =>
          simp [nextLiveCheese?, requestEq, outcomeEq] at advanced
          subst after
          exact request.performLiveCheese_success_wellFormed reason wellFormed outcomeEq

/-- Each successor is passed back to planner.request, so no anchor or live-key carrier is reused. -/
def liveCheeseTicks
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (planner : LiveCheeseResolver (plane := plane) (frame := frame) (position := position) resolver Surface Score)
    (reason : String) :
    Nat → SpatialWorld schema frames spatial entities position →
      Option (SpatialWorld schema frames spatial entities position)
  | 0, world => some world
  | steps + 1, world =>
      match planner.nextLiveCheese? reason world with
      | none => none
      | some after => liveCheeseTicks planner reason steps after

theorem liveCheeseTicks_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {resolver : entities.PlayerResolverToken}
    {Surface : Type u} {Score : Type v}
    (planner : LiveCheeseResolver (plane := plane) (frame := frame) (position := position) resolver Surface Score)
    (reason : String)
    (steps : Nat)
    {before after : SpatialWorld schema frames spatial entities position}
    (wellFormed : WellFormed before)
    (ran : planner.liveCheeseTicks reason steps before = some after) :
    WellFormed after := by
  induction steps generalizing before with
  | zero =>
      simp [liveCheeseTicks] at ran
      subst after
      exact wellFormed
  | succ steps inductionHypothesis =>
      cases advancedEq : planner.nextLiveCheese? reason before with
      | none =>
          simp [liveCheeseTicks, advancedEq] at ran
      | some successor =>
          have successorWellFormed : WellFormed successor :=
            planner.nextLiveCheese?_wellFormed reason wellFormed advancedEq
          have rest : planner.liveCheeseTicks reason steps successor = some after := by
            simpa [liveCheeseTicks, advancedEq] using ran
          exact inductionHypothesis successorWellFormed rest

end LiveCheeseResolver

/--
Generic helper over a fixed command. Accepted live ticks use LiveCheeseResolver so the anchor and
Fin entity carrier are rebuilt from each successor world.
-/
def nextCheese?
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) (before : SpatialWorld schema frames spatial entities position) :
    Option (SpatialWorld schema frames spatial entities position) :=
  match performCheese command reason before with
  | .refused _ _ => none
  | .ok after _ => some after

theorem nextCheese?_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) {before after : SpatialWorld schema frames spatial entities position}
    (wellFormed : WellFormed before)
    (advanced : nextCheese? command reason before = some after) :
    WellFormed after := by
  cases outcomeEq : performCheese command reason before with
  | refused snapshot refusal => simp [nextCheese?, outcomeEq] at advanced
  | ok successor payload =>
      simp [nextCheese?, outcomeEq] at advanced
      subst after
      exact performCheese_success_wellFormed command reason wellFormed outcomeEq

def cheeseTicks
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) : Nat → SpatialWorld schema frames spatial entities position →
      Option (SpatialWorld schema frames spatial entities position)
  | 0, world => some world
  | steps + 1, world =>
      match nextCheese? command reason world with
      | none => none
      | some after => cheeseTicks command reason steps after

theorem cheeseTicks_wellFormed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{u, v, w}}
    {spatial : SpatialRegistry.{u, v, w, x, u, v, w, q} schema frames}
    {entities : EntitySpatialRegistry.{u, v, w, x, q, r} schema frames spatial}
    {plane frame : frames.Id} {position : entities.PositionToken frame}
    {Entity : Type s} {Surface : Type u} {Score : Type v}
    (command : SpatialSpawn.CheeseCommand schema Entity Surface frames plane frame Score)
    (reason : String) (steps : Nat)
    {before after : SpatialWorld schema frames spatial entities position}
    (wellFormed : WellFormed before)
    (ran : cheeseTicks command reason steps before = some after) :
    WellFormed after := by
  induction steps generalizing before with
  | zero =>
      simp [cheeseTicks] at ran
      subst after
      exact wellFormed
  | succ steps inductionHypothesis =>
      cases advancedEq : nextCheese? command reason before with
      | none => simp [cheeseTicks, advancedEq] at ran
      | some successor =>
          have successorWellFormed : WellFormed successor :=
            nextCheese?_wellFormed command reason wellFormed advancedEq
          have rest : cheeseTicks command reason steps successor = some after := by
            simpa [cheeseTicks, advancedEq] using ran
          exact inductionHypothesis successorWellFormed rest

end SpatialWorld

end Ano
