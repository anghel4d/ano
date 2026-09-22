import Ano.SpatialQuery
import Ano.SpatialRegistry
import Ano.Allocation
import Ano.World
import Ano.Affine

namespace Ano.SpatialSpawn

universe u v w x y z p q

/-- The worked spatial batch contains exactly fifty-one copy rows. -/
abbrev CheeseCopies := Fin 51

/- The canonical duplicate-free enumeration is the Std-only cardinality witness for the copy domain. -/
@[simp] theorem cheeseCopies_card :
    (List.finRange 51 : List CheeseCopies).length = 51 := by
  rfl

theorem cheeseCopies_enumerated (copy : CheeseCopies) :
    copy ∈ (List.finRange 51 : List CheeseCopies) :=
  List.mem_finRange copy

/-- A player-relative pattern is admitted only with one selected source and a typed plane-to-world embedding. -/
structure PlayerPattern (Entity : Type x) (frames : FrameSchema.{u, v, w})
    (plane world : frames.Id) where
  private mk ::
  selected : Entity → Prop
  player : Entity
  player_selected : selected player
  player_unique : ∀ other, selected other → other = player
  position : Field Entity (Point frames world)
  offset : Field CheeseCopies (SpatialVector frames plane)
  offset_injective : Function.Injective offset
  embed : OffsetEmbedding frames plane world

/-- A player pattern retaining the exact registry token's genuinely linear refinement. -/
structure LinearPlayerPattern
    {schema : Schema.{u, v, w, p}} {frames : FrameSchema.{u, v, w}}
    (registry : SpatialRegistry schema frames) (affine : AffineFrameBundle frames)
    {K : Type q} (scalars : SemiringLaw K) {plane world : frames.Id}
    (planeModule : ModuleLaw scalars (affine.space plane).vectors)
    (worldModule : ModuleLaw scalars (affine.space world).vectors)
    (token : registry.OffsetMapToken plane world) (Entity : Type x) where
  private mk ::
  pattern : PlayerPattern Entity frames plane world
  embedding : RegisteredLinearAffineEmbedding registry affine scalars
    planeModule worldModule token
  pattern_embed : pattern.embed = registry.offsetMap token

namespace PlayerPattern

/-- Build a player pattern only from a registry-authorized plane-to-world offset map. -/
def registered {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {frames : FrameSchema.{u, v, w}}
    {plane world : frames.Id}
    (registry : SpatialRegistry schema frames)
    (embedding : registry.OffsetMapToken plane world)
    (selected : Entity → Prop) (player : Entity)
    (player_selected : selected player)
    (player_unique : ∀ other, selected other → other = player)
    (position : Field Entity (Point frames world))
    (offset : Field CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    PlayerPattern Entity frames plane world where
  selected := selected
  player := player
  player_selected := player_selected
  player_unique := player_unique
  position := position
  offset := offset
  offset_injective := offset_injective
  embed := registry.offsetMap embedding

/-- Build a player pattern without erasing the registry token's scalar-linearity witness. -/
def registeredLinear {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {frames : FrameSchema.{u, v, w}}
    {plane world : frames.Id}
    {registry : SpatialRegistry schema frames} {affine : AffineFrameBundle frames}
    {K : Type q} {scalars : SemiringLaw K}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {worldModule : ModuleLaw scalars (affine.space world).vectors}
    {token : registry.OffsetMapToken plane world}
    (linear : RegisteredLinearAffineEmbedding registry affine scalars
      planeModule worldModule token)
    (selected : Entity → Prop) (player : Entity)
    (player_selected : selected player)
    (player_unique : ∀ other, selected other → other = player)
    (position : Field Entity (Point frames world))
    (offset : Field CheeseCopies (SpatialVector frames plane))
    (offset_injective : Function.Injective offset) :
    LinearPlayerPattern registry affine scalars planeModule worldModule token Entity where
  pattern := registered registry token selected player player_selected player_unique
    position offset offset_injective
  embedding := linear
  pattern_embed := rfl

/-- Every copy row retains the one certified player source. -/
def source {Entity : Type x} {frames : FrameSchema.{u, v, w}}
    {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) : CheeseCopies → Entity :=
  fun _ => pattern.player

/-- The registered embedding turns one local pattern offset into one world-frame seed point. -/
def seeds {Entity : Type x} {frames : FrameSchema.{u, v, w}}
    {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) :
    Field CheeseCopies (Point frames world) :=
  fun copy => pattern.embed.place (pattern.position pattern.player) (pattern.offset copy)

@[simp] theorem source_apply {Entity : Type x}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) (copy : CheeseCopies) :
    pattern.source copy = pattern.player :=
  rfl

@[simp] theorem seed_apply {Entity : Type x}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) (copy : CheeseCopies) :
    pattern.seeds copy =
      pattern.embed.place (pattern.position pattern.player) (pattern.offset copy) :=
  rfl

theorem source_is_selected {Entity : Type x}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) (copy : CheeseCopies) :
    pattern.selected (pattern.source copy) :=
  pattern.player_selected

theorem every_selected_source_is_player {Entity : Type x}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) {other : Entity}
    (selected : pattern.selected other) : other = pattern.player :=
  pattern.player_unique other selected
/-- The typed embedding and distinct phyllotaxis offsets give distinct world-space seeds. -/
theorem seeds_injective {Entity : Type x}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    (pattern : PlayerPattern Entity frames plane world) :
    Function.Injective pattern.seeds := by
  intro left right sameSeed
  apply pattern.offset_injective
  exact pattern.embed.place_injective
    (pattern.position pattern.player) sameSeed

end PlayerPattern

namespace LinearPlayerPattern

def seeds {schema : Schema.{u, v, w, p}} {frames : FrameSchema.{u, v, w}}
    {registry : SpatialRegistry schema frames} {affine : AffineFrameBundle frames}
    {K : Type q} {scalars : SemiringLaw K} {plane world : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {worldModule : ModuleLaw scalars (affine.space world).vectors}
    {token : registry.OffsetMapToken plane world} {Entity : Type x}
    (linear : LinearPlayerPattern registry affine scalars planeModule worldModule
      token Entity) : Field CheeseCopies (Point frames world) :=
  linear.pattern.seeds

@[simp] theorem seeds_apply
    {schema : Schema.{u, v, w, p}} {frames : FrameSchema.{u, v, w}}
    {registry : SpatialRegistry schema frames} {affine : AffineFrameBundle frames}
    {K : Type q} {scalars : SemiringLaw K} {plane world : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {worldModule : ModuleLaw scalars (affine.space world).vectors}
    {token : registry.OffsetMapToken plane world} {Entity : Type x}
    (linear : LinearPlayerPattern registry affine scalars planeModule worldModule
      token Entity) (copy : CheeseCopies) :
    linear.seeds copy = (registry.offsetMap token).place
      (linear.pattern.position linear.pattern.player) (linear.pattern.offset copy) := by
  change linear.pattern.embed.place
      (linear.pattern.position linear.pattern.player) (linear.pattern.offset copy) = _
  rw [linear.pattern_embed]

/-- Scalar preservation remains available on the exact embedding used by the seed pattern. -/
theorem place_smul
    {schema : Schema.{u, v, w, p}} {frames : FrameSchema.{u, v, w}}
    {registry : SpatialRegistry schema frames} {affine : AffineFrameBundle frames}
    {K : Type q} {scalars : SemiringLaw K} {plane world : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {worldModule : ModuleLaw scalars (affine.space world).vectors}
    {token : registry.OffsetMapToken plane world} {Entity : Type x}
    (linear : LinearPlayerPattern registry affine scalars planeModule worldModule
      token Entity) (anchor : Point frames world) (scalar : K)
    (vector : SpatialVector frames plane) :
    linear.pattern.embed.place anchor (planeModule.smul scalar vector) =
      (affine.space world).vadd anchor
        (worldModule.smul scalar (linear.embedding.embedding.vectorMap vector)) := by
  rw [linear.pattern_embed]
  exact linear.embedding.place_smul anchor scalar vector

theorem seeds_injective
    {schema : Schema.{u, v, w, p}} {frames : FrameSchema.{u, v, w}}
    {registry : SpatialRegistry schema frames} {affine : AffineFrameBundle frames}
    {K : Type q} {scalars : SemiringLaw K} {plane world : frames.Id}
    {planeModule : ModuleLaw scalars (affine.space plane).vectors}
    {worldModule : ModuleLaw scalars (affine.space world).vectors}
    {token : registry.OffsetMapToken plane world} {Entity : Type x}
    (linear : LinearPlayerPattern registry affine scalars planeModule worldModule
      token Entity) : Function.Injective linear.seeds :=
  linear.pattern.seeds_injective

end LinearPlayerPattern

/-- A prepared batch freezes the unique player, all seeds, and all certified support hits before allocation. -/
structure PreparedBatch (Entity : Type x) (Surface : Type y)
    (frames : FrameSchema.{u, v, w}) (plane world : frames.Id)
    (Score : Type z) (projector : SupportProjector Surface frames world Score) where
  pattern : PlayerPattern Entity frames plane world
  resting : RestingPose Surface frames world
  resolved : ResolvedSupportBatch projector pattern.seeds

namespace PreparedBatch

/-- Final entity origins are prototype-specific resting poses, not raw collision points. -/
def positions {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z} {projector : SupportProjector Surface frames world Score}
    (batch : PreparedBatch Entity Surface frames plane world Score projector) :
    Field CheeseCopies (Point frames world) :=
  batch.resolved.positions batch.resting

theorem every_hit_is_best {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z} {projector : SupportProjector Surface frames world Score}
    (batch : PreparedBatch Entity Surface frames plane world Score projector)
    (copy : CheeseCopies) :
    projector.spec.Best (batch.pattern.seeds copy) (batch.resolved.hits copy) :=
  batch.resolved.hit_best copy

theorem every_position_is_supported {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z} {projector : SupportProjector Surface frames world Score}
    (batch : PreparedBatch Entity Surface frames plane world Score projector)
    (copy : CheeseCopies) :
    batch.resting.supported (batch.resolved.hits copy) (batch.positions copy) :=
  batch.resolved.position_supported batch.resting copy

theorem every_copy_has_player_source {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z} {projector : SupportProjector Surface frames world Score}
    (batch : PreparedBatch Entity Surface frames plane world Score projector)
    (copy : CheeseCopies) :
    batch.pattern.source copy = batch.pattern.player :=
  rfl

end PreparedBatch

/-- One cheese command freezes its pattern and support projector from the same input world. -/
structure CheeseCommand (schema : Schema.{u, v, w, p})
    (Entity : Type x) (Surface : Type y)
    (frames : FrameSchema.{u, v, w}) (plane world : frames.Id)
    (Score : Type z) where
  support : SnapshotSupportProjector (World schema) Surface frames world Score
  pattern : World schema → PlayerPattern Entity frames plane world
  /-- This total capability is constructed or refused by host validation before entering the kernel. -/
  resting : RestingPose Surface frames world
  validate : (snapshot : World schema) →
    SupportBatchValidator (support.freeze snapshot) (pattern snapshot).seeds

/-- The fixed planner payload existentially retains the projector frozen from its stored snapshot. -/
structure FrozenPreparedBatch (schema : Schema.{u, v, w, p})
    (Entity : Type x) (Surface : Type y)
    (frames : FrameSchema.{u, v, w}) (plane world : frames.Id)
    (Score : Type z)
    (support : SnapshotSupportProjector (World schema) Surface frames world Score) where
  snapshot : World schema
  projector : SupportProjector Surface frames world Score
  frozen : projector = support.freeze snapshot
  batch : PreparedBatch Entity Surface frames plane world Score projector

namespace FrozenPreparedBatch

/-- Read the already-validated supported centers carried by this frozen command payload. -/
def positions {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    {support : SnapshotSupportProjector (World schema) Surface frames world Score}
    (payload : FrozenPreparedBatch schema Entity Surface frames plane world Score support) :
    Field CheeseCopies (Point frames world) :=
  payload.batch.positions

/-- Every exposed center is supported by the proof-carrying resting pose stored in the payload. -/
theorem every_position_is_supported {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    {support : SnapshotSupportProjector (World schema) Surface frames world Score}
    (payload : FrozenPreparedBatch schema Entity Surface frames plane world Score support)
    (copy : CheeseCopies) :
    payload.batch.resting.supported (payload.batch.resolved.hits copy)
      (payload.positions copy) :=
  payload.batch.every_position_is_supported copy

end FrozenPreparedBatch

namespace CheeseCommand

/-- Validation packages the exact frozen projector and its complete support batch before allocation. -/
def prepare {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (snapshot : World schema) :
    Option (FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support) :=
  (command.validate snapshot).run.map fun resolved =>
    { snapshot := snapshot
      projector := command.support.freeze snapshot
      frozen := rfl
      batch :=
        { pattern := command.pattern snapshot
          resting := command.resting
          resolved := resolved } }

/-- The command rejects exactly when one of its fifty-one frozen-snapshot projector lookups misses. -/
theorem prepare_none_iff {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (snapshot : World schema) :
    command.prepare snapshot = none ↔
      ∃ copy : CheeseCopies,
        (command.support.freeze snapshot).locator.locate
          ((command.pattern snapshot).seeds copy) = none := by
  constructor
  · intro preparedNone
    have validationNone : (command.validate snapshot).run = none := by
      cases validation : (command.validate snapshot).run with
      | none => rfl
      | some resolved =>
          simp [prepare, validation] at preparedNone
    exact (command.validate snapshot).rejects_iff.mp validationNone
  · intro miss
    have validationNone := (command.validate snapshot).rejects_iff.mpr miss
    simp [prepare, validationNone]

end CheeseCommand

/-- Accepted copy rows map to the fresh right-hand segment of the enlarged entity population. -/
def freshKey (oldCount : Nat) : CheeseCopies → Fin (oldCount + 51) :=
  Allocation.freshKey oldCount

theorem freshKey_injective (oldCount : Nat) :
    Function.Injective (freshKey oldCount) :=
  Allocation.freshKey_injective oldCount

theorem freshKey_not_old (oldCount : Nat) (old : Fin oldCount)
    (copy : CheeseCopies) :
    Allocation.oldKey (added := 51) old ≠ freshKey oldCount copy :=
  Allocation.freshKey_not_old old copy

/-- Projection and pose validation finish before this planner emits one structural spawn action. -/
def planner {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String) :
    Planner schema Output :=
  fun snapshot =>
    match prepare snapshot with
    | none => .refuse reason
    | some output => .spawn 51 output

theorem refusal_atomic {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    (snapshot : World schema) (rejected : prepare snapshot = none) :
    perform (planner prepare reason) snapshot = .refused snapshot reason := by
  exact perform_refusal_atomic (planner prepare reason) snapshot reason (by
    simp [planner, rejected])

theorem success_entityCount {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    {snapshot after : World schema} {output : Output}
    (accepted : perform (planner prepare reason) snapshot = .ok after output) :
    after.entityCount = snapshot.entityCount + 51 := by
  cases prepared : prepare snapshot with
  | none => simp [planner, perform, prepared] at accepted
  | some planned =>
      simp [planner, perform, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      rfl

theorem success_fixed {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    {snapshot after : World schema} {output : Output}
    (accepted : perform (planner prepare reason) snapshot = .ok after output) :
    after.fixed = snapshot.fixed := by
  cases prepared : prepare snapshot with
  | none => simp [planner, perform, prepared] at accepted
  | some planned =>
      simp [planner, perform, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      rfl

theorem success_field {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    {snapshot after : World schema} {output : Output}
    (accepted : perform (planner prepare reason) snapshot = .ok after output)
    (field : schema.FieldId) :
    after.fixed field = snapshot.fixed field := by
  rw [success_fixed prepare reason accepted]

theorem success_wellFormed {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    {snapshot after : World schema} {output : Output}
    (wellFormed : WellFormed snapshot)
    (accepted : perform (planner prepare reason) snapshot = .ok after output) :
    WellFormed after :=
  perform_ok_wellFormed (planner prepare reason) wellFormed accepted

theorem successful_ticks_wellFormed
    {schema : Schema.{u, v, w, p}} {Output : Type q}
    (prepare : World schema → Option Output) (reason : String)
    (steps : Nat) {snapshot after : World schema}
    (wellFormed : WellFormed snapshot)
    (ran : ticks (planner prepare reason) steps snapshot = some after) :
    WellFormed after :=
  ticks_wellFormed (planner prepare reason) steps wellFormed ran

namespace CheeseCommand

/-- The command planner validates the frozen snapshot completely before emitting the fifty-one-row spawn. -/
def toPlanner {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) :
    Planner schema (FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support) :=
  planner command.prepare reason

/-- Every frozen support miss refuses with the exact input world. -/
theorem miss_refuses_atomically {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) (snapshot : World schema)
    (miss : ∃ copy : CheeseCopies,
      (command.support.freeze snapshot).locator.locate
        ((command.pattern snapshot).seeds copy) = none) :
    perform (command.toPlanner reason) snapshot = .refused snapshot reason := by
  have rejected := (command.prepare_none_iff snapshot).2 miss
  simpa [toPlanner] using
    (refusal_atomic command.prepare reason snapshot rejected)

/-- A returned prepared payload records exactly the world from which it was frozen. -/
theorem prepared_snapshot_eq {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (snapshot : World schema)
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (prepared : command.prepare snapshot = some payload) :
    payload.snapshot = snapshot := by
  cases validation : (command.validate snapshot).run with
  | none => simp [prepare, validation] at prepared
  | some resolved =>
      simp [prepare, validation] at prepared
      obtain rfl := prepared
      rfl

/-- A returned prepared payload retains the exact pattern computed from its input world. -/
theorem prepared_pattern_eq {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (snapshot : World schema)
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (prepared : command.prepare snapshot = some payload) :
    payload.batch.pattern = command.pattern snapshot := by
  cases validation : (command.validate snapshot).run with
  | none => simp [prepare, validation] at prepared
  | some resolved =>
      simp [prepare, validation] at prepared
      obtain rfl := prepared
      rfl

/-- A returned prepared payload retains the command prototype resting capability. -/
theorem prepared_resting_eq {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (snapshot : World schema)
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (prepared : command.prepare snapshot = some payload) :
    payload.batch.resting = command.resting := by
  cases validation : (command.validate snapshot).run with
  | none => simp [prepare, validation] at prepared
  | some resolved =>
      simp [prepare, validation] at prepared
      obtain rfl := prepared
      rfl

/-- Every accepted cheese command adds exactly fifty-one entity rows. -/
theorem success_adds_exactly_fifty_one {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    after.entityCount = snapshot.entityCount + 51 := by
  simpa [toPlanner] using
    (success_entityCount command.prepare reason accepted)

/-- Every accepted cheese command leaves the complete fixed-field family unchanged. -/
theorem success_preserves_fixed {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    after.fixed = snapshot.fixed := by
  simpa [toPlanner] using (success_fixed command.prepare reason accepted)

/-- Each registered fixed field is extensionally unchanged by an accepted cheese command. -/
theorem success_preserves_fixed_field {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload)
    (field : schema.FieldId) :
    after.fixed field = snapshot.fixed field := by
  simpa [toPlanner] using
    (success_field command.prepare reason accepted field)

/-- Successful output exposes the same snapshot used to freeze the projector and pattern. -/
theorem success_payload_snapshot {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    payload.snapshot = snapshot := by
  cases prepared : command.prepare snapshot with
  | none => simp [toPlanner, planner, perform, prepared] at accepted
  | some planned =>
      have sameSnapshot := command.prepared_snapshot_eq snapshot prepared
      simp [toPlanner, planner, perform, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact sameSnapshot

/-- Successful output exposes the exact pattern computed from the input snapshot. -/
theorem success_payload_pattern {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    payload.batch.pattern = command.pattern snapshot := by
  cases prepared : command.prepare snapshot with
  | none => simp [toPlanner, planner, perform, prepared] at accepted
  | some planned =>
      have samePattern := command.prepared_pattern_eq snapshot prepared
      simp [toPlanner, planner, perform, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact samePattern

/-- Successful output exposes the exact prototype resting capability supplied by the command. -/
theorem success_payload_resting {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    payload.batch.resting = command.resting := by
  cases prepared : command.prepare snapshot with
  | none => simp [toPlanner, planner, perform, prepared] at accepted
  | some planned =>
      have sameResting := command.prepared_resting_eq snapshot prepared
      simp [toPlanner, planner, perform, prepared] at accepted
      obtain ⟨rfl, rfl⟩ := accepted
      exact sameResting

/-- Every position exposed by an accepted payload is supported by its stored resting capability. -/
theorem success_position_is_supported {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (_accepted : perform (command.toPlanner reason) snapshot = .ok after payload)
    (copy : CheeseCopies) :
    payload.batch.resting.supported (payload.batch.resolved.hits copy)
      (payload.positions copy) :=
  payload.every_position_is_supported copy

/-- Successful output exposes the projector frozen from the exact input snapshot. -/
theorem success_projector_frozen_from_input
    {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    payload.projector = command.support.freeze snapshot := by
  calc
    payload.projector = command.support.freeze payload.snapshot := payload.frozen
    _ = command.support.freeze snapshot :=
      congrArg command.support.freeze
        (command.success_payload_snapshot reason accepted)

/-- A successful command step preserves the registered world invariant. -/
theorem success_preserves_wellFormed {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) {snapshot after : World schema}
    {payload : FrozenPreparedBatch schema Entity Surface frames plane world Score
      command.support}
    (wellFormed : WellFormed snapshot)
    (accepted : perform (command.toPlanner reason) snapshot = .ok after payload) :
    WellFormed after := by
  simpa [toPlanner] using
    (success_wellFormed command.prepare reason wellFormed accepted)

/-- Any finite run of successful cheese-command ticks preserves the registered world invariant. -/
theorem successful_repeated_ticks_preserve_wellFormed
    {schema : Schema.{u, v, w, p}}
    {Entity : Type x} {Surface : Type y}
    {frames : FrameSchema.{u, v, w}} {plane world : frames.Id}
    {Score : Type z}
    (command : CheeseCommand schema Entity Surface frames plane world Score)
    (reason : String) (steps : Nat) {snapshot after : World schema}
    (wellFormed : WellFormed snapshot)
    (ran : ticks (command.toPlanner reason) steps snapshot = some after) :
    WellFormed after := by
  simpa [toPlanner] using
    (successful_ticks_wellFormed command.prepare reason steps wellFormed ran)

end CheeseCommand

end Ano.SpatialSpawn
