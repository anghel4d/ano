import Ano.Affine
import Ano.ColumnBundle
import Ano.SpatialNegative

namespace Ano.SpatialAdvancedNegativeWitness

universe u v w x y z p q r s

variable {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
variable {spatial : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}

variable {K : Type r} {weights : WeightLaw K}
variable {authority : SpatialInterpolationRegistry spatial K weights}
variable {habitat : schema.sites.Id} {frame : frames.Id}
variable {token : authority.Token habitat frame}
variable {J : Type s} {positions : Field J (Point frames frame)}

/--
error: Type mismatch
  rows.span.cell
has type
  rows.span.Edge → CellRef schema.sites habitat
but is expected to have type
  Lineage spatial rows.span.Edge (CellRef schema.sites habitat)
-/
#guard_msgs (error, drop info) in
#check fun (rows : RegisteredInterpolationRows spatial authority token positions) =>
  (rows.span.cell : Lineage spatial rows.span.Edge (CellRef schema.sites habitat))

/--
error: Type mismatch
  rows.span.source
has type
  rows.span.Edge → J
but is expected to have type
  Lineage spatial rows.span.Edge J
-/
#guard_msgs (error, drop info) in
#check fun (rows : RegisteredInterpolationRows spatial authority token positions) =>
  (rows.span.source : Lineage spatial rows.span.Edge J)

variable {L : Type q} {affine : AffineFrameBundle frames} {scalars : SemiringLaw L}
variable {source target : frames.Id}
variable {sourceModule : ModuleLaw scalars (affine.space source).vectors}
variable {targetModule : ModuleLaw scalars (affine.space target).vectors}
variable {offsetToken : spatial.OffsetMapToken source target}

/--
error: Type mismatch
  embedding
has type
  AnchoredLinearEmbedding scalars (affine.space source) (affine.space target) sourceModule targetModule
but is expected to have type
  RegisteredLinearAffineEmbedding spatial affine scalars sourceModule targetModule offsetToken
-/
#guard_msgs (error, drop info) in
#check fun
    (embedding : AnchoredLinearEmbedding scalars (affine.space source)
      (affine.space target) sourceModule targetModule) =>
  (embedding : RegisteredLinearAffineEmbedding spatial affine scalars
    sourceModule targetModule offsetToken)

/--
error: Application type mismatch: The argument
  point
has type
  Point SpatialNegativeWitness.frames SpatialNegativeWitness.FrameId.mars3
but is expected to have type
  Point SpatialNegativeWitness.frames SpatialNegativeWitness.FrameId.world3
in the application
  interpolator.resolve point
-/
#guard_msgs (error, drop info) in
#check fun {D : Type} {W : Type} {law : WeightLaw W}
    (interpolator : Interpolator SpatialNegativeWitness.frames
      .world3 D law)
    (point : Point SpatialNegativeWitness.frames .mars3) =>
  interpolator.resolve point

variable {bundleSchema : Schema.{u, v, w, x}}
variable {bundleFrames : FrameSchema.{u, v, w}}
variable {bundleSpatial :
  SpatialRegistry.{u, v, w, x, u, v, w, q} bundleSchema bundleFrames}
variable {entityRegistry :
  EntitySpatialRegistry.{u, v, w, x, q, r} bundleSchema bundleFrames bundleSpatial}
variable {bundleFrame : bundleFrames.Id}
variable {positionToken : entityRegistry.PositionToken bundleFrame}
variable {world : SpatialWorld bundleSchema bundleFrames bundleSpatial
  entityRegistry positionToken}
variable {BundleJ : Type s}

/--
error: Application type mismatch: The argument
  raw
has type
  BundleJ → FieldDomain bundleSchema field
but is expected to have type
  Lineage bundleSpatial BundleJ (FieldDomain bundleSchema field)
in the application
  ColumnBundle.RegisteredInput.fixedField field raw
-/
#guard_msgs (error, drop info) in
#check fun (field : bundleSchema.FieldId)
    (raw : BundleJ → FieldDomain bundleSchema field) =>
  (ColumnBundle.RegisteredInput.fixedField (world := world) field raw :
    ColumnBundle.RegisteredInput world BundleJ
      (ULift.{v} (bundleSchema.Value field)))

end Ano.SpatialAdvancedNegativeWitness
