import Ano.Lineage
import Ano.SpatialQuery
import Ano.Validation

namespace Ano

universe u v w x y z p q r s

namespace LocatedRows

/-- Sample a registered field through the sealed locator-derived lineage. -/
def sample
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) {V : Type s}
    (field : Field (CellRef schema.sites site) V) : Field J V :=
  Lineage.gather (Lineage.located rows) field

@[simp] theorem sample_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) {V : Type s}
    (field : Field (CellRef schema.sites site) V) (row : J) :
    rows.sample field row = field (rows.destination row) :=
  rfl

/-- The partial observation and sealed-lineage gather agree on every successfully located row. -/
theorem sample_agrees_with_locator
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) {V : Type s}
    (field : Field (CellRef schema.sites site) V) (row : J) :
    (registry.situated spatial).locator.sample? positions field row =
      some (rows.sample field row) := by
  exact (registry.situated spatial).locator.sample?_eq_some_of_locate
    positions field row (rows.destination row) (rows.located row)

end LocatedRows

/-- Plain registered write-back additionally certifies that no two located rows share one cell. -/
structure UniqueRegisteredRows
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {site : schema.sites.Id} {frame : frames.Id}
    (spatial : registry.SituatedToken site frame)
    {J : Type r} (positions : Field J (Point frames frame)) where
  rows : LocatedRows registry spatial positions
  destination_injective : Function.Injective rows.destination

namespace UniqueRegisteredRows

/-- Scatter through registered spatial lineage only after proving destination uniqueness. -/
noncomputable def scatter
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : UniqueRegisteredRows registry spatial positions) {V : Type s}
    (base : Field (CellRef schema.sites site) V) (values : Field J V) :
    Field (CellRef schema.sites site) V :=
  Scatter.assign rows.rows.destination base values

@[simp] theorem scatter_hit
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : UniqueRegisteredRows registry spatial positions) {V : Type s}
    (base : Field (CellRef schema.sites site) V) (values : Field J V)
    (row : J) :
    rows.scatter base values (rows.rows.destination row) = values row :=
  Scatter.assign_hit rows.rows.destination rows.destination_injective base values row

theorem scatter_miss
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : UniqueRegisteredRows registry spatial positions) {V : Type s}
    (base : Field (CellRef schema.sites site) V) (values : Field J V)
    (cell : CellRef schema.sites site)
    (miss : ¬ ∃ row, rows.rows.destination row = cell) :
    rows.scatter base values cell = base cell :=
  Scatter.assign_miss rows.rows.destination base values cell miss

end UniqueRegisteredRows

/-- One field value paired with the immutable nominal box capability supplied by the registry. -/
structure BoxedFieldView
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (field : schema.FieldId) where
  boxed : Boxed (FieldDomain schema field)
  values : Field (FieldDomain schema field) (schema.Value field)

namespace SpatialRegistry

/-- Read one fixed field with its schema-resident rank and shape witness. -/
def readBoxedField
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (field : schema.FieldId)
    (tag : registry.BoxToken (schema.fieldSite field))
    (world : World schema) : BoxedFieldView registry field where
  boxed := registry.box tag
  values := world.fixed field

@[simp] theorem readBoxedField_boxed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (field : schema.FieldId)
    (tag : registry.BoxToken (schema.fieldSite field))
    (world : World schema) :
    (registry.readBoxedField field tag world).boxed = registry.box tag :=
  rfl

/-- Spawn changes entity population but leaves the boxed field and its rank/shape witness identical. -/
@[simp] theorem readBoxedField_spawn
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (field : schema.FieldId)
    (tag : registry.BoxToken (schema.fieldSite field))
    (world : World schema) (count : Nat) :
    registry.readBoxedField field tag (world.spawn count) =
      registry.readBoxedField field tag world :=
  rfl

/-- Ordinary value updates may change cells, but the schema-resident box witness is definitionally fixed. -/
@[simp] theorem readBoxedField_update_boxed
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (field : schema.FieldId)
    (tag : registry.BoxToken (schema.fieldSite field))
    (update : OrdinaryUpdate schema) (world : World schema) :
    (registry.readBoxedField field tag (update.apply world)).boxed =
      (registry.readBoxedField field tag world).boxed :=
  rfl

end SpatialRegistry

end Ano
