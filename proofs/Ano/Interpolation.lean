import Ano.SpatialRegistry
import Ano.Effects

namespace Ano

universe u v w x y z p q r s t

/-- Algebraic weight normalization: commutative addition plus a distinguished total weight. -/
structure WeightLaw (K : Type u) where
  sum : Effects.CommMonoidLaw K
  one : K

namespace WeightLaw

def total {K : Type u} (weights : WeightLaw K) {D : Type v}
    (entries : List (D × K)) : K :=
  entries.foldl (fun current entry => weights.sum.op current entry.2)
    weights.sum.identity

end WeightLaw

/-- One finite, nonempty, duplicate-free, algebraically normalized interpolation fiber. -/
structure WeightedSupport {K : Type u} (weights : WeightLaw K) (D : Type v) where
  entries : List (D × K)
  nonempty : Nonempty (Fin entries.length)
  cell_injective : Function.Injective
    (fun slot : Fin entries.length => (entries.get slot).1)
  normalized : weights.total entries = weights.one

namespace WeightedSupport

def entry {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D)
    (slot : Fin support.entries.length) : D × K :=
  support.entries.get slot

def cell {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D)
    (slot : Fin support.entries.length) : D :=
  (support.entry slot).1

def coefficient {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D)
    (slot : Fin support.entries.length) : K :=
  (support.entry slot).2

theorem cells_duplicate_free {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) :
    Function.Injective support.cell :=
  support.cell_injective

theorem sum_coefficients_eq_one {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) :
    weights.total support.entries = weights.one :=
  support.normalized

/-- Nonnegativity is deliberately optional; algebraic normalization alone does not imply it. -/
structure Nonnegative {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) (isNonnegative : K → Prop) : Prop where
  all : ∀ slot, isNonnegative (support.coefficient slot)

/-- A singleton fiber has exactly one unit-weight entry; this says nothing about a metric. -/
def SingletonAt {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) (cell : D) : Prop :=
  support.entries = [(cell, weights.one)]

end WeightedSupport

/-- A locally finite weighted relation, represented as one certified fiber per source row. -/
structure WeightedSpan {K : Type u} (weights : WeightLaw K)
    (X : Type v) (D : Type w) where
  support : X → WeightedSupport weights D

namespace WeightedSpan

/-- Edge rows are the dependent sum of source rows and their finite support slots.
No global edge enumeration is claimed unless the source domain is separately enumerated. -/
abbrev Edge {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) :=
  (row : X) × Fin (span.support row).entries.length

def source {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) : span.Edge → X :=
  fun edge => edge.1

def cell {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) : span.Edge → D :=
  fun edge => (span.support edge.1).cell edge.2

def coefficient {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) : span.Edge → K :=
  fun edge => (span.support edge.1).coefficient edge.2

/-- The finite edge enumeration of one source fiber. -/
def fiberEdges {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) (row : X) : List span.Edge :=
  (List.finRange (span.support row).entries.length).map fun slot => ⟨row, slot⟩

private theorem finRange_nodup (length : Nat) :
    (List.finRange length).Nodup := by
  induction length with
  | zero => simp
  | succ length inductionHypothesis =>
      rw [List.finRange_succ, List.nodup_cons]
      constructor
      · intro member
        simp only [List.mem_map] at member
        obtain ⟨slot, _, equal⟩ := member
        exact Fin.succ_ne_zero slot equal
      · refine inductionHypothesis.map Fin.succ ?_
        intro left right different equal
        exact different (Fin.succ_inj.mp equal)

/-- A local CSR fiber enumerates no edge twice. -/
theorem fiberEdges_nodup {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D) (row : X) :
    (span.fiberEdges row).Nodup := by
  unfold fiberEdges
  rw [List.nodup_iff_pairwise_ne, List.pairwise_map]
  exact (finRange_nodup (span.support row).entries.length).imp (by
    intro left right different equal
    apply different
    exact Fin.ext (congrArg (fun edge : span.Edge => edge.2.val) equal))

/-- Membership is exactly source-fiber membership; no edge of that source is omitted. -/
theorem mem_fiberEdges_iff {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    (row : X) (edge : span.Edge) :
    edge ∈ span.fiberEdges row ↔ span.source edge = row := by
  constructor
  · intro member
    simp only [fiberEdges, List.mem_map] at member
    obtain ⟨slot, _, equal⟩ := member
    rw [← equal]
    rfl
  · intro sameSource
    cases edge with
    | mk edgeRow slot =>
        change edgeRow = row at sameSource
        subst edgeRow
        simp [fiberEdges, List.mem_finRange]

@[simp] theorem source_fiberEdge {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    (row : X) (slot : Fin (span.support row).entries.length) :
    span.source ⟨row, slot⟩ = row :=
  rfl

theorem source_has_edge {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D) (row : X) :
    ∃ edge : span.Edge, span.source edge = row := by
  obtain ⟨slot⟩ := (span.support row).nonempty
  exact ⟨⟨row, slot⟩, rfl⟩

/-- Within one source fiber, a target cell appears at most once.

This does not imply global target-cell injectivity: distinct source fibers may collide, so
weighted spans expose gather and fiber reduction but no unmerged reverse assignment. -/
theorem same_source_same_cell {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {left right : span.Edge}
    (sameSource : span.source left = span.source right)
    (sameCell : span.cell left = span.cell right) : left = right := by
  cases left with
  | mk leftRow leftSlot =>
      cases right with
      | mk rightRow rightSlot =>
          change leftRow = rightRow at sameSource
          subst rightRow
          change (span.support leftRow).cell leftSlot =
            (span.support leftRow).cell rightSlot at sameCell
          have sameSlot := (span.support leftRow).cells_duplicate_free sameCell
          subst rightSlot
          rfl

theorem fiber_normalized {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D) (row : X) :
    weights.total (span.support row).entries = weights.one :=
  (span.support row).normalized

end WeightedSpan

/-- Exact laws needed to scale values and reduce an unordered weighted fiber. -/
structure WeightedValueLaw {K : Type u} (weights : WeightLaw K) (V : Type v) where
  sum : Effects.CommMonoidLaw V
  scale : K → V → V
  one_scale : ∀ value, scale weights.one value = value
  zero_scale : ∀ value, scale weights.sum.identity value = sum.identity
  add_scale : ∀ left right value,
    scale (weights.sum.op left right) value = sum.op (scale left value) (scale right value)
  scale_zero : ∀ weight, scale weight sum.identity = sum.identity
  scale_add : ∀ weight left right,
    scale weight (sum.op left right) = sum.op (scale weight left) (scale weight right)

namespace WeightedSupport

def sample {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) {V : Type w}
    (values : WeightedValueLaw weights V) (field : Field D V) : V :=
  support.entries.foldl
    (fun current entry => values.sum.op current (values.scale entry.2 (field entry.1)))
    values.sum.identity

theorem sample_singleton {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) {V : Type w}
    (values : WeightedValueLaw weights V) (field : Field D V) (cell : D)
    (single : support.SingletonAt cell) : support.sample values field = field cell := by
  unfold SingletonAt at single
  unfold sample
  rw [single]
  simp [values.one_scale, values.sum.leftIdentity]

/-- Algebraic normalization makes interpolation reproduce a constant field. -/
theorem sample_constant {K : Type u} {weights : WeightLaw K} {D : Type v}
    (support : WeightedSupport weights D) {V : Type w}
    (values : WeightedValueLaw weights V) (value : V) :
    support.sample values (fun _ => value) = value := by
  have fold_scaled : ∀ (entries : List (D × K)) (initial : K),
      entries.foldl
          (fun current entry =>
            values.sum.op current (values.scale entry.2 value))
          (values.scale initial value) =
        values.scale
          (entries.foldl
            (fun current entry => weights.sum.op current entry.2) initial)
          value := by
    intro entries
    induction entries with
    | nil =>
        intro initial
        rfl
    | cons entry entries inductionHypothesis =>
        intro initial
        simp only [List.foldl_cons]
        rw [Eq.symm (values.add_scale initial entry.2 value)]
        exact inductionHypothesis (weights.sum.op initial entry.2)
  unfold sample
  rw [Eq.symm (values.zero_scale value)]
  rw [fold_scaled support.entries weights.sum.identity]
  rw [show support.entries.foldl
      (fun current entry => weights.sum.op current entry.2)
      weights.sum.identity = weights.one from support.normalized]
  exact values.one_scale value

end WeightedSupport

namespace WeightedSpan

/-- Gather field values to edge rows through the typed cell leg. -/
def gather {K : Type u} {weights : WeightLaw K} {X : Type v} {D : Type w}
    (span : WeightedSpan weights X D) {V : Type x}
    (field : Field D V) : Field span.Edge V :=
  Field.reindex span.cell field

def weightedGather {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (field : Field D V) : Field span.Edge V :=
  fun edge => values.scale (span.coefficient edge) (span.gather field edge)

def reduceEdges {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (edgeValues : Field span.Edge V) (edges : List span.Edge) : V :=
  edges.foldl (fun current edge => values.sum.op current (edgeValues edge))
    values.sum.identity

def reduceFiber {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (edgeValues : Field span.Edge V) : Field X V :=
  fun row => span.reduceEdges values edgeValues (span.fiberEdges row)

/-- Sampling is edge gather, coefficient scaling, then fiber reduction to one value per source row. -/
def sample {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (field : Field D V) : Field X V :=
  span.reduceFiber values (span.weightedGather values field)

@[simp] theorem gather_apply {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (field : Field D V) (edge : span.Edge) :
    span.gather field edge = field (span.cell edge) :=
  rfl

@[simp] theorem weightedGather_apply {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (field : Field D V) (edge : span.Edge) :
    span.weightedGather values field edge =
      values.scale (span.coefficient edge) (field (span.cell edge)) :=
  rfl

/-- The local CSR slot enumeration contains exactly every support entry, in order. -/
theorem finRange_map_entry {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D) (row : X) :
    (List.finRange (span.support row).entries.length).map
        (span.support row).entry =
      (span.support row).entries := by
  simp [WeightedSupport.entry, List.finRange, Function.comp_def]

/-- List-backed support sampling is exactly edge gather followed by local fiber reduction. -/
theorem support_sample_eq_reduceFiber {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (field : Field D V) (row : X) :
    (span.support row).sample values field =
      span.reduceFiber values (span.weightedGather values field) row := by
  unfold WeightedSupport.sample reduceFiber reduceEdges weightedGather gather
  rw [Eq.symm (span.finRange_map_entry row)]
  simp only [fiberEdges, List.foldl_map]
  rfl

/-- Edge gather and fiber reduction reproduce a constant field pointwise. -/
theorem sample_constant {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V) (value : V) :
    span.sample values (fun _ => value) = fun _ => value := by
  funext row
  change span.reduceFiber values
      (span.weightedGather values (fun _ => value)) row = value
  rw [Eq.symm
    (span.support_sample_eq_reduceFiber values (fun _ => value) row)]
  exact (span.support row).sample_constant values value

@[simp] theorem reduceFiber_apply {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (edgeValues : Field span.Edge V) (row : X) :
    span.reduceFiber values edgeValues row =
      span.reduceEdges values edgeValues (span.fiberEdges row) :=
  rfl

theorem reduceEdges_permutation_independent {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V)
    (edgeValues : Field span.Edge V) {left right : List span.Edge}
    (permutation : left.Perm right) :
    span.reduceEdges values edgeValues left = span.reduceEdges values edgeValues right := by
  apply permutation.foldl_eq'
  intro leftEdge _ rightEdge _ current
  calc
    values.sum.op (values.sum.op current (edgeValues leftEdge)) (edgeValues rightEdge) =
        values.sum.op current
          (values.sum.op (edgeValues leftEdge) (edgeValues rightEdge)) :=
      values.sum.associative current (edgeValues leftEdge) (edgeValues rightEdge)
    _ = values.sum.op current
          (values.sum.op (edgeValues rightEdge) (edgeValues leftEdge)) :=
      congrArg (values.sum.op current)
        (values.sum.commutative (edgeValues leftEdge) (edgeValues rightEdge))
    _ = values.sum.op (values.sum.op current (edgeValues rightEdge))
          (edgeValues leftEdge) :=
      (values.sum.associative current (edgeValues rightEdge) (edgeValues leftEdge)).symm

theorem sample_eq_gather_then_fiber_reduce {K : Type u} {weights : WeightLaw K}
    {X : Type v} {D : Type w} (span : WeightedSpan weights X D)
    {V : Type x} (values : WeightedValueLaw weights V) (field : Field D V) :
    span.sample values field = span.reduceFiber values (span.weightedGather values field) :=
  rfl

end WeightedSpan

/-- A partial deterministic interpolator certified against an independent support relation. -/
structure Interpolator (frames : FrameSchema.{u, v, w}) (frame : frames.Id)
    (D : Type x) {K : Type y} (weights : WeightLaw K) where
  accepts : Point frames frame → WeightedSupport weights D → Prop
  resolve : Point frames frame → Option (WeightedSupport weights D)
  sound : ∀ {point support}, resolve point = some support → accepts point support
  complete : ∀ {point},
    (∃ support, accepts point support) → ∃ support, resolve point = some support
  functional : ∀ {point left right},
    accepts point left → accepts point right → left = right

namespace Interpolator

theorem resolve_eq_some_iff {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    (point : Point frames frame) (support : WeightedSupport weights D) :
    interpolator.resolve point = some support ↔ interpolator.accepts point support := by
  constructor
  · exact interpolator.sound
  · intro accepted
    obtain ⟨chosen, resolved⟩ := interpolator.complete ⟨support, accepted⟩
    have chosenAccepted := interpolator.sound resolved
    have same := interpolator.functional chosenAccepted accepted
    subst chosen
    exact resolved

theorem resolve_eq_none_iff {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights) (point : Point frames frame) :
    interpolator.resolve point = none ↔
      ¬ ∃ support, interpolator.accepts point support := by
  constructor
  · intro missing ⟨support, accepted⟩
    have resolved := (interpolator.resolve_eq_some_iff point support).2 accepted
    rw [missing] at resolved
    cases resolved
  · intro noAccepted
    cases resolved : interpolator.resolve point with
    | none => rfl
    | some support =>
        exact False.elim (noAccepted ⟨support, interpolator.sound resolved⟩)

def samplePoint? {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    {V : Type z} (values : WeightedValueLaw weights V)
    (field : Field D V) (point : Point frames frame) : Option V :=
  (interpolator.resolve point).map fun support => support.sample values field

def sample? {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    {J : Type z} (positions : Field J (Point frames frame))
    {V : Type p} (values : WeightedValueLaw weights V)
    (field : Field D V) : Field J (Option V) :=
  fun row => interpolator.samplePoint? values field (positions row)

end Interpolator

/-- Total frozen interpolation evidence for every row of one position column. -/
structure InterpolatorRows {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    {J : Type z} (positions : Field J (Point frames frame)) where
  support : J → WeightedSupport weights D
  resolved : ∀ row, interpolator.resolve (positions row) = some (support row)

namespace InterpolatorRows

def span {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {K : Type y} {weights : WeightLaw K}
    {interpolator : Interpolator frames frame D weights}
    {J : Type z} {positions : Field J (Point frames frame)}
    (rows : InterpolatorRows interpolator positions) : WeightedSpan weights J D where
  support := rows.support

def sample {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {K : Type y} {weights : WeightLaw K}
    {interpolator : Interpolator frames frame D weights}
    {J : Type z} {positions : Field J (Point frames frame)}
    (rows : InterpolatorRows interpolator positions)
    {V : Type p} (values : WeightedValueLaw weights V)
    (field : Field D V) : Field J V :=
  rows.span.sample values field

theorem sample_agrees_with_partial {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    {interpolator : Interpolator frames frame D weights}
    {J : Type z} {positions : Field J (Point frames frame)}
    (rows : InterpolatorRows interpolator positions)
    {V : Type p} (values : WeightedValueLaw weights V)
    (field : Field D V) (row : J) :
    interpolator.sample? positions values field row = some (rows.sample values field row) := by
  rw [show interpolator.sample? positions values field row =
      some ((rows.support row).sample values field) by
    simp [Interpolator.sample?, Interpolator.samplePoint?, rows.resolved]]
  congr 1
  exact rows.span.support_sample_eq_reduceFiber values field row

end InterpolatorRows

/-- Optional exact-center law; it is not implied by placement or normalization. -/
structure ExactCenterLaw {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    (placement : Placement D frames frame) where
  exact : ∀ cell, ∃ support,
    interpolator.resolve (placement cell) = some support ∧ support.SingletonAt cell

namespace ExactCenterLaw

theorem samplePoint?_placement {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} {K : Type y} {weights : WeightLaw K}
    {interpolator : Interpolator frames frame D weights}
    {placement : Placement D frames frame}
    (law : ExactCenterLaw interpolator placement)
    {V : Type z} (values : WeightedValueLaw weights V)
    (field : Field D V) (cell : D) :
    interpolator.samplePoint? values field (placement cell) = some (field cell) := by
  obtain ⟨support, resolved, single⟩ := law.exact cell
  simp [Interpolator.samplePoint?, resolved, support.sample_singleton values field cell single]

end ExactCenterLaw

/-- Optional agreement with a declared locator; no metric-nearest claim is inferred. -/
structure SingletonLocatorLaw {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    {D : Type x} {K : Type y} {weights : WeightLaw K}
    (interpolator : Interpolator frames frame D weights)
    (locator : Locator frames frame D) where
  resolve_iff : ∀ point support,
    interpolator.resolve point = some support ↔
      ∃ cell, locator.locate point = some cell ∧ support.SingletonAt cell

/-- Registry authority for interpolation is a sibling capability indexed by one spatial registry. -/
structure SpatialInterpolationRegistry
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (spatial : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (K : Type r) (weights : WeightLaw K) where
  Token : schema.sites.Id → frames.Id → Type s
  interpolator : {habitat : schema.sites.Id} → {frame : frames.Id} →
    Token habitat frame →
      Interpolator frames frame (CellRef schema.sites habitat) weights

/-- A registered interpolation result cannot be built from a raw span without token-specific resolution. -/
structure RegisteredInterpolationRows
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (spatial : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {K : Type r} {weights : WeightLaw K}
    (authority : SpatialInterpolationRegistry spatial K weights)
    {habitat : schema.sites.Id} {frame : frames.Id}
    (token : authority.Token habitat frame)
    {J : Type t} (positions : Field J (Point frames frame)) where
  private mk ::
  rows : InterpolatorRows (authority.interpolator token) positions

namespace RegisteredInterpolationRows

/-- Seal rows only after resolving them against the exact registry token. -/
def ofResolved
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {spatial : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry spatial K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type t} {positions : Field J (Point frames frame)}
    (rows : InterpolatorRows (authority.interpolator token) positions) :
    RegisteredInterpolationRows spatial authority token positions where
  rows := rows

def span
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {spatial : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry spatial K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type t} {positions : Field J (Point frames frame)}
    (registered : RegisteredInterpolationRows spatial authority token positions) :
    WeightedSpan weights J (CellRef schema.sites habitat) :=
  registered.rows.span

end RegisteredInterpolationRows

end Ano
