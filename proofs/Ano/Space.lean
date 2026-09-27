import Ano.Field

namespace Ano

universe u v w x

/-- A finite rectangular carrier. Registered habitat identity remains separate. -/
abbrev Box {rank : Nat} (shape : Fin rank → Nat) :=
  (axis : Fin rank) → Fin (shape axis)

/-- The canonical free integer lattice carrier of a given rank. -/
abbrev IntegerLattice (rank : Nat) := Fin rank → Int

/-- Include bounded box coordinates into the canonical integer lattice. -/
def Box.latticePoint {rank : Nat} {shape : Fin rank → Nat}
    (site : Box shape) : IntegerLattice rank :=
  fun axis => Int.ofNat (site axis).val

/-- A nominal domain certified to be a finite rectangular window. -/
structure Boxed (D : Type u) where
  rank : Nat
  shape : Fin rank → Nat
  sites : Iso (Box shape) D

namespace Boxed

/-- Recover the shape coordinate of a nominal site. -/
def coordinate {D : Type u} (boxed : Boxed D) : D → Box boxed.shape :=
  boxed.sites.invFun

@[simp] theorem coordinate_site {D : Type u} (boxed : Boxed D)
    (site : Box boxed.shape) :
    boxed.coordinate (boxed.sites site) = site :=
  boxed.sites.leftInv site

@[simp] theorem site_coordinate {D : Type u} (boxed : Boxed D) (site : D) :
    boxed.sites (boxed.coordinate site) = site :=
  boxed.sites.rightInv site

end Boxed

/-- Names ambient frames while keeping their point and displacement carriers independent. -/
structure FrameSchema where
  Id : Type u
  PointCarrier : Id → Type v
  VectorCarrier : Id → Type w

/-- A point whose frame is part of its type. -/
structure Point (frames : FrameSchema.{u, v, w}) (frame : frames.Id) where
  coordinate : frames.PointCarrier frame

/-- A displacement whose frame is part of its type. -/
structure SpatialVector (frames : FrameSchema.{u, v, w}) (frame : frames.Id) where
  coordinate : frames.VectorCarrier frame

/-- An explicitly declared map between points in two frames. -/
structure PointMap (frames : FrameSchema.{u, v, w})
    (source target : frames.Id) where
  toFun : Point frames source → Point frames target

/-- A registered local-vector embedding places offsets around an anchor in a target frame. -/
structure OffsetEmbedding (frames : FrameSchema.{u, v, w})
    (source target : frames.Id) where
  place : Point frames target → SpatialVector frames source → Point frames target
  place_injective : ∀ anchor, Function.Injective (place anchor)

namespace PointMap

instance {frames : FrameSchema.{u, v, w}} {source target : frames.Id} :
    CoeFun (PointMap frames source target)
      (fun _ => Point frames source → Point frames target) :=
  ⟨PointMap.toFun⟩

def refl (frames : FrameSchema.{u, v, w}) (frame : frames.Id) :
    PointMap frames frame frame where
  toFun := id

/-- Composition applies the left map first and the right map second. -/
def trans {frames : FrameSchema.{u, v, w}} {source middle target : frames.Id}
    (left : PointMap frames source middle)
    (right : PointMap frames middle target) :
    PointMap frames source target where
  toFun := right ∘ left

@[simp] theorem refl_apply {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (point : Point frames frame) :
    refl frames frame point = point :=
  rfl

@[simp] theorem trans_apply {frames : FrameSchema.{u, v, w}}
    {source middle target : frames.Id}
    (left : PointMap frames source middle)
    (right : PointMap frames middle target)
    (point : Point frames source) :
    trans left right point = right (left point) :=
  rfl

@[simp] theorem refl_trans {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} (change : PointMap frames source target) :
    trans (refl frames source) change = change := by
  cases change
  rfl

@[simp] theorem trans_refl {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} (change : PointMap frames source target) :
    trans change (refl frames target) = change := by
  cases change
  rfl

@[simp] theorem trans_assoc {frames : FrameSchema.{u, v, w}}
    {a b c d : frames.Id}
    (ab : PointMap frames a b) (bc : PointMap frames b c)
    (cd : PointMap frames c d) :
    trans (trans ab bc) cd = trans ab (trans bc cd) := by
  cases ab
  cases bc
  cases cd
  rfl

end PointMap

/-- Placement is independent of the field placed over the domain. -/
abbrev Placement (D : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) :=
  D → Point frames frame

namespace Placement

/-- Change the ambient frame of a placement through an explicit point map. -/
def map {D : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    (change : PointMap frames source target)
    (placement : Placement D frames source) :
    Placement D frames target :=
  change ∘ placement

@[simp] theorem map_apply {D : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    (change : PointMap frames source target)
    (placement : Placement D frames source) (site : D) :
    map change placement site = change (placement site) :=
  rfl

@[simp] theorem map_refl {D : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} (placement : Placement D frames frame) :
    map (PointMap.refl frames frame) placement = placement :=
  rfl

@[simp] theorem map_trans {D : Type x} {frames : FrameSchema.{u, v, w}}
    {source middle target : frames.Id}
    (left : PointMap frames source middle)
    (right : PointMap frames middle target)
    (placement : Placement D frames source) :
    map (PointMap.trans left right) placement = map right (map left placement) :=
  rfl

end Placement

/-- A partial deterministic locator certified against an independent spatial relation. -/
structure Locator (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) (D : Type x) where
  accepts : Point frames frame → D → Prop
  locate : Point frames frame → Option D
  sound : ∀ {point site}, locate point = some site → accepts point site
  complete : ∀ {point},
    (∃ site, accepts point site) → ∃ site, locate point = some site
  functional : ∀ {point left right},
    accepts point left → accepts point right → left = right

namespace Locator

theorem locate_eq_some_iff {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} (locator : Locator frames frame D)
    (point : Point frames frame) (site : D) :
    locator.locate point = some site ↔ locator.accepts point site := by
  constructor
  · exact locator.sound
  · intro accepted
    obtain ⟨chosen, located⟩ := locator.complete ⟨site, accepted⟩
    have chosenAccepted : locator.accepts point chosen := locator.sound located
    have same : chosen = site := locator.functional chosenAccepted accepted
    subst chosen
    exact located

theorem locate_eq_none_iff {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} {D : Type x} (locator : Locator frames frame D)
    (point : Point frames frame) :
    locator.locate point = none ↔ ¬ ∃ site, locator.accepts point site := by
  constructor
  · intro missing existsAccepted
    obtain ⟨site, accepted⟩ := existsAccepted
    have located : locator.locate point = some site :=
      (locator.locate_eq_some_iff point site).2 accepted
    rw [missing] at located
    cases located
  · intro noAccepted
    cases located : locator.locate point with
    | none => rfl
    | some site =>
        exact False.elim (noAccepted ⟨site, locator.sound located⟩)

end Locator

/-- A coherent placement and locator for one domain in one ambient frame. -/
structure Situated (D : Type x) (frames : FrameSchema.{u, v, w})
    (frame : frames.Id) where
  placement : Placement D frames frame
  locator : Locator frames frame D
  locate_placement : ∀ site, locator.locate (placement site) = some site

namespace Situated

theorem placement_injective {D : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} (situated : Situated D frames frame) :
    Function.Injective situated.placement := by
  intro left right samePoint
  have sameLocation := congrArg situated.locator.locate samePoint
  rw [situated.locate_placement left, situated.locate_placement right] at sameLocation
  exact Option.some.inj sameLocation

theorem accepts_placement {D : Type x} {frames : FrameSchema.{u, v, w}}
    {frame : frames.Id} (situated : Situated D frames frame) (site : D) :
    situated.locator.accepts (situated.placement site) site :=
  situated.locator.sound (situated.locate_placement site)

end Situated

end Ano
