import Ano.Space
import Ano.World

namespace Ano

universe u v w x y z p q r

/-- A nominal reference to one cell of one registered habitat. -/
abbrev CellRef (sites : SiteSchema.{u, v}) (habitat : sites.Id) :=
  Site sites habitat

/-- Spatial capabilities granted by the host registry for one world schema. -/
structure SpatialRegistry
    (schema : Schema.{u, v, w, x})
    (frames : FrameSchema.{y, z, p}) where
  PositionToken : (field : schema.FieldId) → frames.Id → Type q
  positionView : {field : schema.FieldId} → {frame : frames.Id} →
    PositionToken field frame →
      Iso (schema.Value field) (Point frames frame)
  BoxToken : schema.sites.Id → Type q
  box : {site : schema.sites.Id} →
    BoxToken site → Boxed (CellRef schema.sites site)
  LineageToken : schema.sites.Id → schema.sites.Id → Type q
  lineageMap : {source target : schema.sites.Id} →
    LineageToken source target →
      CellRef schema.sites source → CellRef schema.sites target
  FrameMapToken : frames.Id → frames.Id → Type q
  frameMap : {source target : frames.Id} →
    FrameMapToken source target → PointMap frames source target
  OffsetMapToken : frames.Id → frames.Id → Type q
  offsetMap : {source target : frames.Id} →
    OffsetMapToken source target → OffsetEmbedding frames source target
  SituatedToken : schema.sites.Id → frames.Id → Type q
  situated : {site : schema.sites.Id} → {frame : frames.Id} →
    SituatedToken site frame → Situated (CellRef schema.sites site) frames frame

/-- A spatial schema keeps all frame and spatial authority outside mutable world state. -/
structure SpatialSchema where
  frames : FrameSchema.{y, z, p}
  base : Schema.{u, v, w, x}
  registry : SpatialRegistry.{u, v, w, x, y, z, p, q} base frames

namespace SpatialRegistry

/-- Decode a registry-tagged foreign field carrier as frame-indexed points. -/
def readPosition
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {field : schema.FieldId} {frame : frames.Id}
    (tag : registry.PositionToken field frame)
    (column : Field (FieldDomain schema field) (schema.Value field)) :
    Field (FieldDomain schema field) (Point frames frame) :=
  fun row => registry.positionView tag (column row)

@[simp] theorem readPosition_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {field : schema.FieldId} {frame : frames.Id}
    (tag : registry.PositionToken field frame)
    (column : Field (FieldDomain schema field) (schema.Value field))
    (row : FieldDomain schema field) :
    registry.readPosition tag column row = registry.positionView tag (column row) :=
  rfl

/-- Reframe a point column only through a registry-authorized point map. -/
def reframe
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : frames.Id} (tag : registry.FrameMapToken source target)
    {J : Type r} (points : Field J (Point frames source)) :
    Field J (Point frames target) :=
  fun row => registry.frameMap tag (points row)

@[simp] theorem reframe_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : frames.Id} (tag : registry.FrameMapToken source target)
    {J : Type r} (points : Field J (Point frames source)) (row : J) :
    registry.reframe tag points row = registry.frameMap tag (points row) :=
  rfl
/-- Place a local offset column around one target-frame anchor through a registered embedding. -/
def placeOffsets
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : frames.Id} (tag : registry.OffsetMapToken source target)
    (anchor : Point frames target) {J : Type r}
    (offsets : Field J (SpatialVector frames source)) :
    Field J (Point frames target) :=
  fun row => (registry.offsetMap tag).place anchor (offsets row)

@[simp] theorem placeOffsets_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : frames.Id} (tag : registry.OffsetMapToken source target)
    (anchor : Point frames target) {J : Type r}
    (offsets : Field J (SpatialVector frames source)) (row : J) :
    registry.placeOffsets tag anchor offsets row =
      (registry.offsetMap tag).place anchor (offsets row) :=
  rfl

/-- Distinct local offsets remain distinct after the authorized anchored embedding. -/
theorem placeOffsets_injective
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : frames.Id} (tag : registry.OffsetMapToken source target)
    (anchor : Point frames target) {J : Type r}
    (offsets : Field J (SpatialVector frames source))
    (offsetsInjective : Function.Injective offsets) :
    Function.Injective (registry.placeOffsets tag anchor offsets) := by
  intro left right samePoint
  apply offsetsInjective
  exact (registry.offsetMap tag).place_injective anchor samePoint

end SpatialRegistry

/-- Frozen successful locations obtained through one registered situated habitat. -/
structure LocatedRows
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {site : schema.sites.Id} {frame : frames.Id}
    (spatial : registry.SituatedToken site frame)
    {J : Type r} (positions : Field J (Point frames frame)) where
  destination : J → CellRef schema.sites site
  located : ∀ row,
    (registry.situated spatial).locator.locate (positions row) =
      some (destination row)

namespace LocatedRows

/-- Every frozen destination satisfies the registered locator relation. -/
theorem destination_accepted
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) (row : J) :
    (registry.situated spatial).locator.accepts
      (positions row) (rows.destination row) :=
  (registry.situated spatial).locator.sound (rows.located row)

/-- Convert frozen destination cells back to points through the same situated habitat. -/
def placed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) :
    Field J (Point frames frame) :=
  fun row => (registry.situated spatial).placement (rows.destination row)

@[simp] theorem placed_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) (row : J) :
    rows.placed row =
      (registry.situated spatial).placement (rows.destination row) :=
  rfl

@[simp] theorem locate_placed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) (row : J) :
    (registry.situated spatial).locator.locate (rows.placed row) =
      some (rows.destination row) :=
  (registry.situated spatial).locate_placement (rows.destination row)

end LocatedRows

end Ano
