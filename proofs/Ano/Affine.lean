import Ano.SpatialRegistry

namespace Ano

universe u v w x y z p q

/-- The exact additive-commutative-group equations required of one vector carrier. -/
structure AddCommGroupLaw (V : Type u) where
  zero : V
  add : V → V → V
  neg : V → V
  add_assoc : ∀ left middle right,
    add (add left middle) right = add left (add middle right)
  add_comm : ∀ left right, add left right = add right left
  zero_add : ∀ value, add zero value = value
  add_zero : ∀ value, add value zero = value
  neg_add : ∀ value, add (neg value) value = zero
  add_neg : ∀ value, add value (neg value) = zero

namespace AddCommGroupLaw

/-- Subtraction derived from addition and additive inverse. -/
def sub {V : Type u} (law : AddCommGroupLaw V) (left right : V) : V :=
  law.add left (law.neg right)

end AddCommGroupLaw

/-- One nominal frame equipped with a vector group and a simply transitive point action. -/
structure AffineFrame (frames : FrameSchema.{u, v, w}) (frame : frames.Id) where
  vectors : AddCommGroupLaw (SpatialVector frames frame)
  vadd : Point frames frame → SpatialVector frames frame → Point frames frame
  vsub : Point frames frame → Point frames frame → SpatialVector frames frame
  vadd_zero : ∀ point, vadd point vectors.zero = point
  vadd_add : ∀ point left right,
    vadd (vadd point left) right = vadd point (vectors.add left right)
  vsub_vadd : ∀ point vector, vsub (vadd point vector) point = vector
  vadd_vsub : ∀ base target, vadd base (vsub target base) = target

namespace AffineFrame

@[simp] theorem add_zero {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame) :
    space.vadd point space.vectors.zero = point :=
  space.vadd_zero point

theorem add_add {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame)
    (left right : SpatialVector frames frame) :
    space.vadd (space.vadd point left) right =
      space.vadd point (space.vectors.add left right) :=
  space.vadd_add point left right

@[simp] theorem sub_add {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame)
    (vector : SpatialVector frames frame) :
    space.vsub (space.vadd point vector) point = vector :=
  space.vsub_vadd point vector

@[simp] theorem add_sub {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (base target : Point frames frame) :
    space.vadd base (space.vsub target base) = target :=
  space.vadd_vsub base target

@[simp] theorem sub_self {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame) :
    space.vsub point point = space.vectors.zero := by
  calc
    space.vsub point point =
        space.vsub (space.vadd point space.vectors.zero) point := by
          rw [space.vadd_zero]
    _ = space.vectors.zero := space.vsub_vadd point space.vectors.zero

/-- Translation from a fixed point is injective because subtraction recovers the vector. -/
theorem vadd_injective {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame) :
    Function.Injective (space.vadd point) := by
  intro left right samePoint
  have recovered := congrArg (fun target => space.vsub target point) samePoint
  change space.vsub (space.vadd point left) point =
    space.vsub (space.vadd point right) point at recovered
  rw [space.vsub_vadd point left, space.vsub_vadd point right] at recovered
  exact recovered

end AffineFrame

/-- A family of affine-space witnesses equips every nominal frame, without changing its carrier. -/
structure AffineFrameBundle (frames : FrameSchema.{u, v, w}) where
  space : (frame : frames.Id) → AffineFrame frames frame

/-- A homomorphism of the declared additive vector groups. -/
structure AdditiveVectorMap {A : Type u} {B : Type v}
    (source : AddCommGroupLaw A) (target : AddCommGroupLaw B) where
  toFun : A → B
  map_zero : toFun source.zero = target.zero
  map_add : ∀ left right,
    toFun (source.add left right) = target.add (toFun left) (toFun right)

namespace AdditiveVectorMap

instance {A : Type u} {B : Type v}
    {source : AddCommGroupLaw A} {target : AddCommGroupLaw B} :
    CoeFun (AdditiveVectorMap source target) (fun _ => A → B) :=
  ⟨AdditiveVectorMap.toFun⟩

@[ext] theorem ext {A : Type u} {B : Type v}
    {source : AddCommGroupLaw A} {target : AddCommGroupLaw B}
    (left right : AdditiveVectorMap source target)
    (same : ∀ value, left value = right value) : left = right := by
  cases left with
  | mk leftFun leftZero leftAdd =>
      cases right with
      | mk rightFun rightZero rightAdd =>
          have sameFun : leftFun = rightFun := funext same
          subst rightFun
          rfl

def refl {A : Type u} (law : AddCommGroupLaw A) :
    AdditiveVectorMap law law where
  toFun := id
  map_zero := rfl
  map_add := fun _ _ => rfl

/-- Composition applies the left map first and the right map second. -/
def trans {A : Type u} {B : Type v} {C : Type w}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B} {c : AddCommGroupLaw C}
    (left : AdditiveVectorMap a b) (right : AdditiveVectorMap b c) :
    AdditiveVectorMap a c where
  toFun := right ∘ left
  map_zero := by
    change right.toFun (left.toFun a.zero) = c.zero
    rw [left.map_zero, right.map_zero]
  map_add := by
    intro first second
    change right.toFun (left.toFun (a.add first second)) =
      c.add (right.toFun (left.toFun first)) (right.toFun (left.toFun second))
    rw [left.map_add, right.map_add]

@[simp] theorem refl_apply {A : Type u} (law : AddCommGroupLaw A) (value : A) :
    refl law value = value :=
  rfl

@[simp] theorem trans_apply {A : Type u} {B : Type v} {C : Type w}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B} {c : AddCommGroupLaw C}
    (left : AdditiveVectorMap a b) (right : AdditiveVectorMap b c) (value : A) :
    trans left right value = right (left value) :=
  rfl

@[simp] theorem refl_trans {A : Type u} {B : Type v}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    (map : AdditiveVectorMap a b) : trans (refl a) map = map := by
  ext value
  rfl

@[simp] theorem trans_refl {A : Type u} {B : Type v}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    (map : AdditiveVectorMap a b) : trans map (refl b) = map := by
  ext value
  rfl

@[simp] theorem trans_assoc {A : Type u} {B : Type v} {C : Type w} {D : Type x}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    {c : AddCommGroupLaw C} {d : AddCommGroupLaw D}
    (ab : AdditiveVectorMap a b) (bc : AdditiveVectorMap b c)
    (cd : AdditiveVectorMap c d) :
    trans (trans ab bc) cd = trans ab (trans bc cd) := by
  ext value
  rfl

end AdditiveVectorMap

/-- The exact semiring equations needed to state module and linear-map laws without Mathlib. -/
structure SemiringLaw (K : Type u) where
  zero : K
  one : K
  add : K → K → K
  mul : K → K → K
  add_assoc : ∀ a b c, add (add a b) c = add a (add b c)
  add_comm : ∀ a b, add a b = add b a
  zero_add : ∀ a, add zero a = a
  add_zero : ∀ a, add a zero = a
  mul_assoc : ∀ a b c, mul (mul a b) c = mul a (mul b c)
  one_mul : ∀ a, mul one a = a
  mul_one : ∀ a, mul a one = a
  left_distrib : ∀ a b c, mul a (add b c) = add (mul a b) (mul a c)
  right_distrib : ∀ a b c, mul (add a b) c = add (mul a c) (mul b c)
  zero_mul : ∀ a, mul zero a = zero
  mul_zero : ∀ a, mul a zero = zero

/-- A module-shaped action over an explicit scalar semiring and additive vector group. -/
structure ModuleLaw {K : Type u} {V : Type v}
    (scalars : SemiringLaw K) (vectors : AddCommGroupLaw V) where
  smul : K → V → V
  one_smul : ∀ vector, smul scalars.one vector = vector
  mul_smul : ∀ left right vector,
    smul (scalars.mul left right) vector = smul left (smul right vector)
  add_smul : ∀ left right vector,
    smul (scalars.add left right) vector =
      vectors.add (smul left vector) (smul right vector)
  zero_smul : ∀ vector, smul scalars.zero vector = vectors.zero
  smul_add : ∀ scalar left right,
    smul scalar (vectors.add left right) =
      vectors.add (smul scalar left) (smul scalar right)
  smul_zero : ∀ scalar, smul scalar vectors.zero = vectors.zero

/-- A vector map preserving both addition and the declared scalar action. -/
structure LinearVectorMap {K : Type u} {A : Type v} {B : Type w}
    (scalars : SemiringLaw K)
    (sourceVectors : AddCommGroupLaw A) (targetVectors : AddCommGroupLaw B)
    (sourceModule : ModuleLaw scalars sourceVectors)
    (targetModule : ModuleLaw scalars targetVectors)
    extends AdditiveVectorMap sourceVectors targetVectors where
  map_smul : ∀ scalar vector,
    toFun (sourceModule.smul scalar vector) = targetModule.smul scalar (toFun vector)

namespace LinearVectorMap

instance {K : Type u} {A : Type v} {B : Type w}
    {scalars : SemiringLaw K}
    {sourceVectors : AddCommGroupLaw A} {targetVectors : AddCommGroupLaw B}
    {sourceModule : ModuleLaw scalars sourceVectors}
    {targetModule : ModuleLaw scalars targetVectors} :
    CoeFun (LinearVectorMap scalars sourceVectors targetVectors sourceModule targetModule)
      (fun _ => A → B) :=
  ⟨fun map => map.toFun⟩
@[ext] theorem ext {K : Type u} {A : Type v} {B : Type w}
    {scalars : SemiringLaw K}
    {sourceVectors : AddCommGroupLaw A} {targetVectors : AddCommGroupLaw B}
    {sourceModule : ModuleLaw scalars sourceVectors}
    {targetModule : ModuleLaw scalars targetVectors}
    (left right : LinearVectorMap scalars sourceVectors targetVectors
      sourceModule targetModule)
    (same : ∀ vector, left vector = right vector) : left = right := by
  cases left with
  | mk leftAdd leftSmul =>
      cases right with
      | mk rightAdd rightSmul =>
          have sameAdd : leftAdd = rightAdd :=
            AdditiveVectorMap.ext leftAdd rightAdd (fun vector => same vector)
          subst rightAdd
          rfl


def refl {K : Type u} {A : Type v} (scalars : SemiringLaw K)
    (vectors : AddCommGroupLaw A) (module : ModuleLaw scalars vectors) :
    LinearVectorMap scalars vectors vectors module module where
  toFun := id
  map_zero := rfl
  map_add := fun _ _ => rfl
  map_smul := fun _ _ => rfl

def trans {K : Type u} {A : Type v} {B : Type w} {C : Type x}
    {scalars : SemiringLaw K}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B} {c : AddCommGroupLaw C}
    {ma : ModuleLaw scalars a} {mb : ModuleLaw scalars b}
    {mc : ModuleLaw scalars c}
    (left : LinearVectorMap scalars a b ma mb)
    (right : LinearVectorMap scalars b c mb mc) :
    LinearVectorMap scalars a c ma mc where
  toFun := right ∘ left
  map_zero := by
    change right.toFun (left.toFun a.zero) = c.zero
    rw [left.map_zero, right.map_zero]
  map_add := by
    intro first second
    change right.toFun (left.toFun (a.add first second)) =
      c.add (right.toFun (left.toFun first)) (right.toFun (left.toFun second))
    rw [left.map_add, right.map_add]
  map_smul := by
    intro scalar vector
    change right.toFun (left.toFun (ma.smul scalar vector)) =
      mc.smul scalar (right.toFun (left.toFun vector))
    rw [left.map_smul, right.map_smul]

@[simp] theorem refl_apply {K : Type u} {A : Type v}
    (scalars : SemiringLaw K) (vectors : AddCommGroupLaw A)
    (module : ModuleLaw scalars vectors) (value : A) :
    refl scalars vectors module value = value :=
  rfl

@[simp] theorem trans_apply {K : Type u} {A : Type v} {B : Type w} {C : Type x}
    {scalars : SemiringLaw K}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B} {c : AddCommGroupLaw C}
    {ma : ModuleLaw scalars a} {mb : ModuleLaw scalars b}
    {mc : ModuleLaw scalars c}
    (left : LinearVectorMap scalars a b ma mb)
    (right : LinearVectorMap scalars b c mb mc) (value : A) :
    trans left right value = right (left value) :=
  rfl
@[simp] theorem refl_trans {K : Type u} {A : Type v} {B : Type w}
    {scalars : SemiringLaw K}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    {ma : ModuleLaw scalars a} {mb : ModuleLaw scalars b}
    (map : LinearVectorMap scalars a b ma mb) :
    trans (refl scalars a ma) map = map := by
  apply ext
  intro vector
  rfl

@[simp] theorem trans_refl {K : Type u} {A : Type v} {B : Type w}
    {scalars : SemiringLaw K}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    {ma : ModuleLaw scalars a} {mb : ModuleLaw scalars b}
    (map : LinearVectorMap scalars a b ma mb) :
    trans map (refl scalars b mb) = map := by
  apply ext
  intro vector
  rfl

@[simp] theorem trans_assoc {K : Type u} {A : Type v} {B : Type w}
    {C : Type x} {D : Type y} {scalars : SemiringLaw K}
    {a : AddCommGroupLaw A} {b : AddCommGroupLaw B}
    {c : AddCommGroupLaw C} {d : AddCommGroupLaw D}
    {ma : ModuleLaw scalars a} {mb : ModuleLaw scalars b}
    {mc : ModuleLaw scalars c} {md : ModuleLaw scalars d}
    (ab : LinearVectorMap scalars a b ma mb)
    (bc : LinearVectorMap scalars b c mb mc)
    (cd : LinearVectorMap scalars c d mc md) :
    trans (trans ab bc) cd = trans ab (trans bc cd) := by
  apply ext
  intro vector
  rfl


end LinearVectorMap

/-- An additive-affine point map; it makes no scalar-linearity claim. -/
structure AffineMap {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    (sourceSpace : AffineFrame frames source) (targetSpace : AffineFrame frames target) where
  pointMap : Point frames source → Point frames target
  vectorMap : AdditiveVectorMap sourceSpace.vectors targetSpace.vectors
  map_vadd : ∀ point vector,
    pointMap (sourceSpace.vadd point vector) =
      targetSpace.vadd (pointMap point) (vectorMap vector)

namespace AffineMap

instance {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target} :
    CoeFun (AffineMap sourceSpace targetSpace)
      (fun _ => Point frames source → Point frames target) :=
  ⟨AffineMap.pointMap⟩

def refl {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) : AffineMap space space where
  pointMap := id
  vectorMap := AdditiveVectorMap.refl space.vectors
  map_vadd := fun _ _ => rfl

/-- Composition applies the left affine map first and the right affine map second. -/
def trans {frames : FrameSchema.{u, v, w}} {a b c : frames.Id}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c}
    (left : AffineMap aSpace bSpace) (right : AffineMap bSpace cSpace) :
    AffineMap aSpace cSpace where
  pointMap := right ∘ left
  vectorMap := AdditiveVectorMap.trans left.vectorMap right.vectorMap
  map_vadd := by
    intro point vector
    calc
      right (left (aSpace.vadd point vector)) =
          right (bSpace.vadd (left point) (left.vectorMap vector)) :=
        congrArg right (left.map_vadd point vector)
      _ = cSpace.vadd (right (left point))
          (right.vectorMap (left.vectorMap vector)) :=
        right.map_vadd (left point) (left.vectorMap vector)

@[ext] theorem ext {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (left right : AffineMap sourceSpace targetSpace)
    (samePoint : ∀ point, left point = right point)
    (sameVector : ∀ vector, left.vectorMap vector = right.vectorMap vector) :
    left = right := by
  cases left with
  | mk leftPoint leftVector leftLaw =>
      cases right with
      | mk rightPoint rightVector rightLaw =>
          have pointEq : leftPoint = rightPoint := funext samePoint
          have vectorEq : leftVector = rightVector :=
            AdditiveVectorMap.ext leftVector rightVector sameVector
          subst rightPoint
          subst rightVector
          rfl

@[simp] theorem refl_apply {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) (point : Point frames frame) :
    refl space point = point :=
  rfl

@[simp] theorem trans_apply {frames : FrameSchema.{u, v, w}} {a b c : frames.Id}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c}
    (left : AffineMap aSpace bSpace) (right : AffineMap bSpace cSpace)
    (point : Point frames a) : trans left right point = right (left point) :=
  rfl

@[simp] theorem refl_trans {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (map : AffineMap sourceSpace targetSpace) : trans (refl sourceSpace) map = map := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

@[simp] theorem trans_refl {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (map : AffineMap sourceSpace targetSpace) : trans map (refl targetSpace) = map := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

@[simp] theorem trans_assoc {frames : FrameSchema.{u, v, w}} {a b c d : frames.Id}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c} {dSpace : AffineFrame frames d}
    (ab : AffineMap aSpace bSpace) (bc : AffineMap bSpace cSpace)
    (cd : AffineMap cSpace dSpace) :
    trans (trans ab bc) cd = trans ab (trans bc cd) := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

end AffineMap

/-- A genuinely `K`-affine map: its displacement map preserves the declared scalar action. -/
structure LinearAffineMap {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} (scalars : SemiringLaw K)
    (sourceSpace : AffineFrame frames source) (targetSpace : AffineFrame frames target)
    (sourceModule : ModuleLaw scalars sourceSpace.vectors)
    (targetModule : ModuleLaw scalars targetSpace.vectors) where
  pointMap : Point frames source → Point frames target
  vectorMap : LinearVectorMap scalars sourceSpace.vectors targetSpace.vectors
    sourceModule targetModule
  map_vadd : ∀ point vector,
    pointMap (sourceSpace.vadd point vector) =
      targetSpace.vadd (pointMap point) (vectorMap vector)

namespace LinearAffineMap

instance {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors} :
    CoeFun (LinearAffineMap scalars sourceSpace targetSpace sourceModule targetModule)
      (fun _ => Point frames source → Point frames target) :=
  ⟨LinearAffineMap.pointMap⟩

/-- Forget scalar preservation while retaining the exact point action and additive compatibility. -/
def toAffineMap {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (map : LinearAffineMap scalars sourceSpace targetSpace sourceModule targetModule) :
    AffineMap sourceSpace targetSpace where
  pointMap := map.pointMap
  vectorMap := map.vectorMap.toAdditiveVectorMap
  map_vadd := map.map_vadd

@[simp] theorem toAffineMap_apply {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (map : LinearAffineMap scalars sourceSpace targetSpace sourceModule targetModule)
    (point : Point frames source) : map.toAffineMap point = map point :=
  rfl

theorem map_smul {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (map : LinearAffineMap scalars sourceSpace targetSpace sourceModule targetModule)
    (scalar : K) (vector : SpatialVector frames source) :
    map.vectorMap (sourceModule.smul scalar vector) =
      targetModule.smul scalar (map.vectorMap vector) :=
  map.vectorMap.map_smul scalar vector

def refl {K : Type x} {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (scalars : SemiringLaw K) (space : AffineFrame frames frame)
    (module : ModuleLaw scalars space.vectors) :
    LinearAffineMap scalars space space module module where
  pointMap := id
  vectorMap := LinearVectorMap.refl scalars space.vectors module
  map_vadd := fun _ _ => rfl

/-- Composition preserves both point-action compatibility and scalar linearity. -/
def trans {K : Type x} {frames : FrameSchema.{u, v, w}} {a b c : frames.Id}
    {scalars : SemiringLaw K}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c}
    {aModule : ModuleLaw scalars aSpace.vectors}
    {bModule : ModuleLaw scalars bSpace.vectors}
    {cModule : ModuleLaw scalars cSpace.vectors}
    (left : LinearAffineMap scalars aSpace bSpace aModule bModule)
    (right : LinearAffineMap scalars bSpace cSpace bModule cModule) :
    LinearAffineMap scalars aSpace cSpace aModule cModule where
  pointMap := right ∘ left
  vectorMap := LinearVectorMap.trans left.vectorMap right.vectorMap
  map_vadd := by
    intro point vector
    calc
      right (left (aSpace.vadd point vector)) =
          right (bSpace.vadd (left point) (left.vectorMap vector)) :=
        congrArg right (left.map_vadd point vector)
      _ = cSpace.vadd (right (left point))
          (right.vectorMap (left.vectorMap vector)) :=
        right.map_vadd (left point) (left.vectorMap vector)

@[ext] theorem ext {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (left right : LinearAffineMap scalars sourceSpace targetSpace
      sourceModule targetModule)
    (samePoint : ∀ point, left point = right point)
    (sameVector : ∀ vector, left.vectorMap vector = right.vectorMap vector) :
    left = right := by
  cases left with
  | mk leftPoint leftVector leftLaw =>
      cases right with
      | mk rightPoint rightVector rightLaw =>
          have pointEq : leftPoint = rightPoint := funext samePoint
          have vectorEq : leftVector = rightVector :=
            LinearVectorMap.ext leftVector rightVector sameVector
          subst rightPoint
          subst rightVector
          rfl

@[simp] theorem refl_trans {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (map : LinearAffineMap scalars sourceSpace targetSpace
      sourceModule targetModule) :
    trans (refl scalars sourceSpace sourceModule) map = map := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

@[simp] theorem trans_refl {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (map : LinearAffineMap scalars sourceSpace targetSpace
      sourceModule targetModule) :
    trans map (refl scalars targetSpace targetModule) = map := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

@[simp] theorem trans_assoc {K : Type x} {frames : FrameSchema.{u, v, w}}
    {a b c d : frames.Id} {scalars : SemiringLaw K}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c} {dSpace : AffineFrame frames d}
    {aModule : ModuleLaw scalars aSpace.vectors}
    {bModule : ModuleLaw scalars bSpace.vectors}
    {cModule : ModuleLaw scalars cSpace.vectors}
    {dModule : ModuleLaw scalars dSpace.vectors}
    (ab : LinearAffineMap scalars aSpace bSpace aModule bModule)
    (bc : LinearAffineMap scalars bSpace cSpace bModule cModule)
    (cd : LinearAffineMap scalars cSpace dSpace cModule dModule) :
    trans (trans ab bc) cd = trans ab (trans bc cd) := by
  apply ext
  · intro point
    rfl
  · intro vector
    rfl

end LinearAffineMap


/-- An affine equivalence carries separately witnessed inverses on points and vectors. -/
structure AffineEquiv {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    (sourceSpace : AffineFrame frames source) (targetSpace : AffineFrame frames target) where
  toMap : AffineMap sourceSpace targetSpace
  invMap : AffineMap targetSpace sourceSpace
  left_point : ∀ point, invMap (toMap point) = point
  right_point : ∀ point, toMap (invMap point) = point
  left_vector : ∀ vector, invMap.vectorMap (toMap.vectorMap vector) = vector
  right_vector : ∀ vector, toMap.vectorMap (invMap.vectorMap vector) = vector

namespace AffineEquiv

def refl {frames : FrameSchema.{u, v, w}} {frame : frames.Id}
    (space : AffineFrame frames frame) : AffineEquiv space space where
  toMap := AffineMap.refl space
  invMap := AffineMap.refl space
  left_point := fun _ => rfl
  right_point := fun _ => rfl
  left_vector := fun _ => rfl
  right_vector := fun _ => rfl

def symm {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (equiv : AffineEquiv sourceSpace targetSpace) : AffineEquiv targetSpace sourceSpace where
  toMap := equiv.invMap
  invMap := equiv.toMap
  left_point := equiv.right_point
  right_point := equiv.left_point
  left_vector := equiv.right_vector
  right_vector := equiv.left_vector

def trans {frames : FrameSchema.{u, v, w}} {a b c : frames.Id}
    {aSpace : AffineFrame frames a} {bSpace : AffineFrame frames b}
    {cSpace : AffineFrame frames c}
    (left : AffineEquiv aSpace bSpace) (right : AffineEquiv bSpace cSpace) :
    AffineEquiv aSpace cSpace where
  toMap := AffineMap.trans left.toMap right.toMap
  invMap := AffineMap.trans right.invMap left.invMap
  left_point := by
    intro point
    change left.invMap (right.invMap (right.toMap (left.toMap point))) = point
    rw [right.left_point, left.left_point]
  right_point := by
    intro point
    change right.toMap (left.toMap (left.invMap (right.invMap point))) = point
    rw [left.right_point, right.right_point]
  left_vector := by
    intro vector
    change left.invMap.vectorMap
      (right.invMap.vectorMap (right.toMap.vectorMap (left.toMap.vectorMap vector))) = vector
    rw [right.left_vector, left.left_vector]
  right_vector := by
    intro vector
    change right.toMap.vectorMap
      (left.toMap.vectorMap (left.invMap.vectorMap (right.invMap.vectorMap vector))) = vector
    rw [left.right_vector, right.right_vector]

end AffineEquiv

/-- An injective affine placement of source-frame vectors around a target-frame anchor. -/
structure AnchoredAffineEmbedding {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    (sourceSpace : AffineFrame frames source) (targetSpace : AffineFrame frames target) where
  vectorMap : AdditiveVectorMap sourceSpace.vectors targetSpace.vectors
  vector_injective : Function.Injective vectorMap

namespace AnchoredAffineEmbedding

def place {frames : FrameSchema.{u, v, w}} {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (embedding : AnchoredAffineEmbedding sourceSpace targetSpace)
    (anchor : Point frames target) (offset : SpatialVector frames source) :
    Point frames target :=
  targetSpace.vadd anchor (embedding.vectorMap offset)

@[simp] theorem place_zero {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (embedding : AnchoredAffineEmbedding sourceSpace targetSpace)
    (anchor : Point frames target) :
    embedding.place anchor sourceSpace.vectors.zero = anchor := by
  rw [place, embedding.vectorMap.map_zero, targetSpace.vadd_zero]

theorem place_add {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (embedding : AnchoredAffineEmbedding sourceSpace targetSpace)
    (anchor : Point frames target) (left right : SpatialVector frames source) :
    embedding.place anchor (sourceSpace.vectors.add left right) =
      targetSpace.vadd (embedding.place anchor left) (embedding.vectorMap right) := by
  simp only [place, embedding.vectorMap.map_add]
  exact (targetSpace.vadd_add anchor (embedding.vectorMap left)
    (embedding.vectorMap right)).symm

theorem place_injective {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    (embedding : AnchoredAffineEmbedding sourceSpace targetSpace)
    (anchor : Point frames target) :
    Function.Injective (embedding.place anchor) := by
  intro left right samePoint
  apply embedding.vector_injective
  exact targetSpace.vadd_injective anchor samePoint

end AnchoredAffineEmbedding

/-- An injective anchored embedding whose vector map is genuinely `K`-linear. -/
structure AnchoredLinearEmbedding {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} (scalars : SemiringLaw K)
    (sourceSpace : AffineFrame frames source) (targetSpace : AffineFrame frames target)
    (sourceModule : ModuleLaw scalars sourceSpace.vectors)
    (targetModule : ModuleLaw scalars targetSpace.vectors) where
  vectorMap : LinearVectorMap scalars sourceSpace.vectors targetSpace.vectors
    sourceModule targetModule
  vector_injective : Function.Injective vectorMap

namespace AnchoredLinearEmbedding

def toAnchoredAffineEmbedding {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule) :
    AnchoredAffineEmbedding sourceSpace targetSpace where
  vectorMap := embedding.vectorMap.toAdditiveVectorMap
  vector_injective := embedding.vector_injective

def place {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule)
    (anchor : Point frames target) (offset : SpatialVector frames source) :
    Point frames target :=
  targetSpace.vadd anchor (embedding.vectorMap offset)

@[simp] theorem place_zero {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule) (anchor : Point frames target) :
    embedding.place anchor sourceSpace.vectors.zero = anchor := by
  rw [place, embedding.vectorMap.map_zero, targetSpace.vadd_zero]

theorem place_add {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule) (anchor : Point frames target)
    (left right : SpatialVector frames source) :
    embedding.place anchor (sourceSpace.vectors.add left right) =
      targetSpace.vadd (embedding.place anchor left) (embedding.vectorMap right) :=
  embedding.toAnchoredAffineEmbedding.place_add anchor left right

theorem place_smul {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule) (anchor : Point frames target)
    (scalar : K) (vector : SpatialVector frames source) :
    embedding.place anchor (sourceModule.smul scalar vector) =
      targetSpace.vadd anchor (targetModule.smul scalar (embedding.vectorMap vector)) := by
  rw [place, embedding.vectorMap.map_smul]

theorem place_injective {K : Type x} {frames : FrameSchema.{u, v, w}}
    {source target : frames.Id} {scalars : SemiringLaw K}
    {sourceSpace : AffineFrame frames source} {targetSpace : AffineFrame frames target}
    {sourceModule : ModuleLaw scalars sourceSpace.vectors}
    {targetModule : ModuleLaw scalars targetSpace.vectors}
    (embedding : AnchoredLinearEmbedding scalars sourceSpace targetSpace
      sourceModule targetModule) (anchor : Point frames target) :
    Function.Injective (embedding.place anchor) :=
  embedding.toAnchoredAffineEmbedding.place_injective anchor

end AnchoredLinearEmbedding


/-- An existing registry offset token refined by an affine embedding with the same action. -/
structure RegisteredAffineEmbedding
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (affine : AffineFrameBundle frames) {source target : frames.Id}
    (token : registry.OffsetMapToken source target) where
  embedding : AnchoredAffineEmbedding (affine.space source) (affine.space target)
  agrees : ∀ anchor offset,
    (registry.offsetMap token).place anchor offset = embedding.place anchor offset

namespace RegisteredAffineEmbedding

theorem place_zero
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {source target : frames.Id}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredAffineEmbedding registry affine token)
    (anchor : Point frames target) :
    (registry.offsetMap token).place anchor (affine.space source).vectors.zero = anchor := by
  rw [registered.agrees]
  exact registered.embedding.place_zero anchor

theorem place_add
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {source target : frames.Id}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredAffineEmbedding registry affine token)
    (anchor : Point frames target) (left right : SpatialVector frames source) :
    (registry.offsetMap token).place anchor
        ((affine.space source).vectors.add left right) =
      (affine.space target).vadd ((registry.offsetMap token).place anchor left)
        (registered.embedding.vectorMap right) := by
  calc
    (registry.offsetMap token).place anchor
        ((affine.space source).vectors.add left right) =
        registered.embedding.place anchor
          ((affine.space source).vectors.add left right) := registered.agrees anchor _
    _ = (affine.space target).vadd (registered.embedding.place anchor left)
          (registered.embedding.vectorMap right) :=
      registered.embedding.place_add anchor left right
    _ = (affine.space target).vadd ((registry.offsetMap token).place anchor left)
          (registered.embedding.vectorMap right) := by
      rw [registered.agrees anchor left]

theorem place_injective
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {source target : frames.Id}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredAffineEmbedding registry affine token)
    (anchor : Point frames target) :
    Function.Injective ((registry.offsetMap token).place anchor) := by
  intro left right samePoint
  apply registered.embedding.place_injective anchor
  calc
    registered.embedding.place anchor left =
        (registry.offsetMap token).place anchor left := (registered.agrees anchor left).symm
    _ = (registry.offsetMap token).place anchor right := samePoint
    _ = registered.embedding.place anchor right := registered.agrees anchor right

end RegisteredAffineEmbedding

/-- One registry offset token refined by a genuinely `K`-linear anchored embedding. -/
structure RegisteredLinearAffineEmbedding
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (affine : AffineFrameBundle frames) {K : Type q} (scalars : SemiringLaw K)
    {source target : frames.Id}
    (sourceModule : ModuleLaw scalars (affine.space source).vectors)
    (targetModule : ModuleLaw scalars (affine.space target).vectors)
    (token : registry.OffsetMapToken source target) where
  embedding : AnchoredLinearEmbedding scalars (affine.space source)
    (affine.space target) sourceModule targetModule
  agrees : ∀ anchor offset,
    (registry.offsetMap token).place anchor offset = embedding.place anchor offset

namespace RegisteredLinearAffineEmbedding

/-- Forget scalar preservation while retaining the same authorized offset action. -/
def toRegisteredAffineEmbedding
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {source target : frames.Id}
    {sourceModule : ModuleLaw scalars (affine.space source).vectors}
    {targetModule : ModuleLaw scalars (affine.space target).vectors}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredLinearAffineEmbedding registry affine scalars
      sourceModule targetModule token) :
    RegisteredAffineEmbedding registry affine token where
  embedding := registered.embedding.toAnchoredAffineEmbedding
  agrees := registered.agrees

theorem place_smul
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {source target : frames.Id}
    {sourceModule : ModuleLaw scalars (affine.space source).vectors}
    {targetModule : ModuleLaw scalars (affine.space target).vectors}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredLinearAffineEmbedding registry affine scalars
      sourceModule targetModule token)
    (anchor : Point frames target) (scalar : K)
    (vector : SpatialVector frames source) :
    (registry.offsetMap token).place anchor (sourceModule.smul scalar vector) =
      (affine.space target).vadd anchor
        (targetModule.smul scalar (registered.embedding.vectorMap vector)) := by
  calc
    (registry.offsetMap token).place anchor (sourceModule.smul scalar vector) =
        registered.embedding.place anchor (sourceModule.smul scalar vector) :=
      registered.agrees anchor _
    _ = (affine.space target).vadd anchor
          (targetModule.smul scalar (registered.embedding.vectorMap vector)) :=
      registered.embedding.place_smul anchor scalar vector

theorem place_injective
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {affine : AffineFrameBundle frames} {K : Type q} {scalars : SemiringLaw K}
    {source target : frames.Id}
    {sourceModule : ModuleLaw scalars (affine.space source).vectors}
    {targetModule : ModuleLaw scalars (affine.space target).vectors}
    {token : registry.OffsetMapToken source target}
    (registered : RegisteredLinearAffineEmbedding registry affine scalars
      sourceModule targetModule token) (anchor : Point frames target) :
    Function.Injective ((registry.offsetMap token).place anchor) :=
  registered.toRegisteredAffineEmbedding.place_injective anchor

end RegisteredLinearAffineEmbedding


end Ano
