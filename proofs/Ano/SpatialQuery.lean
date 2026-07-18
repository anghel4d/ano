import Ano.Space
import Ano.Effects

namespace Ano

universe u v w x y z

namespace Locator

/-- Sample a field through a partial point-to-domain locator without discarding the source row. -/
def sample? {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} (locator : Locator frames frame D) {J : Type y}
    (positions : Field J (Point frames frame)) {V : Type z}
    (field : Field D V) : Field J (Option V) :=
  fun row => (locator.locate (positions row)).map field

theorem sample?_eq_some_of_locate {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} (locator : Locator frames frame D)
    {J : Type y} (positions : Field J (Point frames frame)) {V : Type z}
    (field : Field D V) (row : J) (site : D)
    (located : locator.locate (positions row) = some site) :
    locator.sample? positions field row = some (field site) := by
  simp [sample?, located]

theorem sample?_eq_none_of_missing {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} (locator : Locator frames frame D)
    {J : Type y} (positions : Field J (Point frames frame)) {V : Type z}
    (field : Field D V) (row : J)
    (missing : locator.locate (positions row) = none) :
    locator.sample? positions field row = none := by
  simp [sample?, missing]

end Locator

/-- A total proof that every row in one frozen position column resolved through one locator. -/
structure LocatorRows {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} (locator : Locator frames frame D) {J : Type y}
    (positions : Field J (Point frames frame)) where
  destination : J → D
  located : ∀ row, locator.locate (positions row) = some (destination row)

namespace LocatorRows

theorem accepted {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {locator : Locator frames frame D} {J : Type y}
    {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions) (row : J) :
    locator.accepts (positions row) (rows.destination row) :=
  locator.sound (rows.located row)

/-- Gather a stored field through the certified point-to-domain destinations. -/
def sample {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {locator : Locator frames frame D} {J : Type y}
    {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions) {V : Type z}
    (field : Field D V) : Field J V :=
  Field.reindex rows.destination field

@[simp] theorem sample_apply {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions) {V : Type z}
    (field : Field D V) (row : J) :
    rows.sample field row = field (rows.destination row) :=
  rfl

theorem sample_agrees_with_partial {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions) {V : Type z}
    (field : Field D V) (row : J) :
    locator.sample? positions field row = some (rows.sample field row) := by
  exact locator.sample?_eq_some_of_locate positions field row
    (rows.destination row) (rows.located row)

/-- Read the declared placement of each located destination; this transforms values, not row lineage. -/
def placed {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {locator : Locator frames frame D} {J : Type y}
    {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions)
    (placement : Placement D frames frame) : Field J (Point frames frame) :=
  fun row => placement (rows.destination row)

@[simp] theorem placed_apply {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : LocatorRows locator positions)
    (placement : Placement D frames frame) (row : J) :
    rows.placed placement row = placement (rows.destination row) :=
  rfl

end LocatorRows

/-- Plain write-back is available only when the located destination is injective. -/
structure UniqueLocatorRows {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} (locator : Locator frames frame D)
    {J : Type y} (positions : Field J (Point frames frame)) where
  rows : LocatorRows locator positions
  destination_injective : Function.Injective rows.destination

namespace UniqueLocatorRows

/-- Scatter values through the same certified destinations used for spatial lookup. -/
noncomputable def scatter {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : UniqueLocatorRows locator positions) {V : Type z}
    (base : Field D V) (values : Field J V) : Field D V :=
  Scatter.assign rows.rows.destination base values

@[simp] theorem scatter_hit {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : UniqueLocatorRows locator positions) {V : Type z}
    (base : Field D V) (values : Field J V) (row : J) :
    rows.scatter base values (rows.rows.destination row) = values row :=
  Scatter.assign_hit rows.rows.destination rows.destination_injective base values row

theorem scatter_miss {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : UniqueLocatorRows locator positions) {V : Type z}
    (base : Field D V) (values : Field J V) (site : D)
    (miss : ¬ ∃ row, rows.rows.destination row = site) :
    rows.scatter base values site = base site :=
  Scatter.assign_miss rows.rows.destination base values site miss

theorem scatter_deterministic {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {locator : Locator frames frame D}
    {J : Type y} {positions : Field J (Point frames frame)}
    (rows : UniqueLocatorRows locator positions) {V : Type z}
    (base : Field D V) (values : Field J V) :
    ∃ out, Scatter.Satisfies rows.rows.destination base values out ∧
      ∀ other, Scatter.Satisfies rows.rows.destination base values other → other = out :=
  Scatter.injective_scatter_deterministic rows.rows.destination
    rows.destination_injective base values

end UniqueLocatorRows

/-- A collision surface hit retains its nominal feature and all geometric values in one frame. -/
structure SurfaceHit (Surface : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) where
  surface : Surface
  point : Point frames frame
  normal : SpatialVector frames frame

/-- The independent mathematical relation used to define one nearest-support policy. -/
structure SupportSpec (Surface : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) (Score : Type y) where
  candidate : Point frames frame → SurfaceHit Surface frames frame → Prop
  score : Point frames frame → SurfaceHit Surface frames frame → Score
  betterOrEqual : Score → Score → Prop
  score_transitive : ∀ {left middle right},
    betterOrEqual left middle → betterOrEqual middle right →
      betterOrEqual left right
  score_antisymmetric : ∀ {left right},
    betterOrEqual left right → betterOrEqual right left → left = right
  score_total : ∀ left right,
    betterOrEqual left right ∨ betterOrEqual right left
  stableKey : Surface → Nat
  tie_unique : ∀ {seed left right},
    candidate seed left → candidate seed right →
    score seed left = score seed right →
    stableKey left.surface = stableKey right.surface → left = right

namespace SupportSpec

/-- A best hit is admissible, minimum by the declared score, and least by semantic key on score ties. -/
def Best {Surface : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {Score : Type y}
    (spec : SupportSpec Surface frames frame Score)
    (seed : Point frames frame) (hit : SurfaceHit Surface frames frame) : Prop :=
  spec.candidate seed hit ∧
    ∀ alternative, spec.candidate seed alternative →
      spec.betterOrEqual (spec.score seed hit) (spec.score seed alternative) ∧
      (spec.score seed hit = spec.score seed alternative →
        spec.stableKey hit.surface ≤ spec.stableKey alternative.surface)

/-- The score order and semantic tie key make the declared best candidate unique. -/
theorem best_functional {Surface : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {Score : Type y}
    (spec : SupportSpec Surface frames frame Score)
    {seed : Point frames frame}
    {left right : SurfaceHit Surface frames frame}
    (leftBest : spec.Best seed left) (rightBest : spec.Best seed right) :
    left = right := by
  have leftScoreLe := (leftBest.2 right rightBest.1).1
  have rightScoreLe := (rightBest.2 left leftBest.1).1
  have sameScore := spec.score_antisymmetric leftScoreLe rightScoreLe
  have leftKeyLe := (leftBest.2 right rightBest.1).2 sameScore
  have rightKeyLe := (rightBest.2 left leftBest.1).2 sameScore.symm
  have sameKey := Nat.le_antisymm leftKeyLe rightKeyLe
  exact spec.tie_unique leftBest.1 rightBest.1 sameScore sameKey

end SupportSpec

/-- A registered support query is certified against `Best`, rather than trusted as an opaque raycast. -/
structure SupportProjector (Surface : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) (Score : Type y) where
  spec : SupportSpec Surface frames frame Score
  locator : Locator frames frame (SurfaceHit Surface frames frame)
  accepts_iff_best : ∀ seed hit,
    locator.accepts seed hit ↔ spec.Best seed hit

/-- A dynamic spatial service yields one proof-carrying projector frozen to each snapshot. -/
structure SnapshotSupportProjector (Snapshot : Type z) (Surface : Type x)
    (frames : FrameSchema.{u, v, w}) (frame : frames.Id) (Score : Type y) where
  freeze : Snapshot → SupportProjector Surface frames frame Score

namespace SupportProjector

theorem locate_eq_some_iff_best {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    (seed : Point frames frame) (hit : SurfaceHit Surface frames frame) :
    projector.locator.locate seed = some hit ↔ projector.spec.Best seed hit := by
  rw [projector.locator.locate_eq_some_iff]
  exact projector.accepts_iff_best seed hit

theorem locate_eq_none_iff_no_best {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    (seed : Point frames frame) :
    projector.locator.locate seed = none ↔
      ¬ ∃ hit, projector.spec.Best seed hit := by
  rw [projector.locator.locate_eq_none_iff]
  constructor
  · intro noneBest ⟨hit, best⟩
    exact noneBest ⟨hit, (projector.accepts_iff_best seed hit).2 best⟩
  · intro noneAccepted ⟨hit, accepted⟩
    exact noneAccepted ⟨hit, (projector.accepts_iff_best seed hit).1 accepted⟩

theorem located_is_best {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    {seed : Point frames frame} {hit : SurfaceHit Surface frames frame}
    (located : projector.locator.locate seed = some hit) :
    projector.spec.Best seed hit :=
  (projector.locate_eq_some_iff_best seed hit).1 located

theorem best_functional {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    {seed : Point frames frame} {left right : SurfaceHit Surface frames frame}
    (leftBest : projector.spec.Best seed left)
    (rightBest : projector.spec.Best seed right) : left = right :=
  projector.spec.best_functional leftBest rightBest

end SupportProjector

/-- A prototype-specific resting pose proves support separately from database destination uniqueness. -/
structure RestingPose (Surface : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) where
  center : SurfaceHit Surface frames frame → Point frames frame
  supported : SurfaceHit Surface frames frame → Point frames frame → Prop
  center_supported : ∀ hit, supported hit (center hit)

/-- An all-or-nothing batch retains one certified hit for every frozen seed row. -/
structure ResolvedSupportBatch {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    {J : Type z} (seeds : Field J (Point frames frame)) where
  hits : Field J (SurfaceHit Surface frames frame)
  projected : ∀ row, projector.locator.locate (seeds row) = some (hits row)

namespace ResolvedSupportBatch

/-- Convert every certified surface hit into the prototype's supported center position. -/
def positions {Surface : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {Score : Type y}
    {projector : SupportProjector Surface frames frame Score}
    {J : Type z} {seeds : Field J (Point frames frame)}
    (batch : ResolvedSupportBatch projector seeds)
    (resting : RestingPose Surface frames frame) : Field J (Point frames frame) :=
  fun row => resting.center (batch.hits row)

theorem hit_best {Surface : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {Score : Type y}
    {projector : SupportProjector Surface frames frame Score}
    {J : Type z} {seeds : Field J (Point frames frame)}
    (batch : ResolvedSupportBatch projector seeds) (row : J) :
    projector.spec.Best (seeds row) (batch.hits row) :=
  projector.located_is_best (batch.projected row)

theorem position_supported {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    {projector : SupportProjector Surface frames frame Score}
    {J : Type z} {seeds : Field J (Point frames frame)}
    (batch : ResolvedSupportBatch projector seeds)
    (resting : RestingPose Surface frames frame) (row : J) :
    resting.supported (batch.hits row) (batch.positions resting row) :=
  resting.center_supported (batch.hits row)

end ResolvedSupportBatch

/-- A validator is complete when it rejects exactly the batches containing at least one projection miss. -/
structure SupportBatchValidator {Surface : Type x}
    {frames : FrameSchema.{u, v, w}} {frame : frames.Id} {Score : Type y}
    (projector : SupportProjector Surface frames frame Score)
    {J : Type z} (seeds : Field J (Point frames frame)) where
  run : Option (ResolvedSupportBatch projector seeds)
  rejects_iff : run = none ↔
    ∃ row, projector.locator.locate (seeds row) = none

end Ano
