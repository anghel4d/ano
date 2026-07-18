import Ano.SpatialRegistry
import Ano.Spawn
import Ano.Interpolation

namespace Ano

universe u v w x y z p q r s t

/--
Registry-indexed semantic row lineage. The private constructor prevents physical layouts or
arbitrary equal-cardinality equivalences from becoming accepted spatial lineage.
-/
structure Lineage
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (J : Type r) (I : Type s) where
  private mk ::
  toFun : J → I

namespace Lineage

instance
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {J : Type r} {I : Type s} :
    CoeFun (Lineage registry J I) (fun _ => J → I) :=
  ⟨Lineage.toFun⟩

/-- Identity lineage is available on every current row domain. -/
def refl
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (D : Type r) : Lineage registry D D where
  toFun := id

/-- Compose semantic lineage without exposing a raw-function constructor. -/
def comp
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {I : Type r} {J : Type s} {K : Type t}
    (outer : Lineage registry J I) (inner : Lineage registry K J) :
    Lineage registry K I where
  toFun := outer ∘ inner

/-- A direct cross-habitat arrow exists only when the registry supplies its token. -/
def registered
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : schema.sites.Id}
    (token : registry.LineageToken source target) :
    Lineage registry (CellRef schema.sites source) (CellRef schema.sites target) where
  toFun := registry.lineageMap token

/-- Selection rows retain lineage through their frozen subtype inclusion. -/
def selection
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {D : Type r} (selected : Selection D) :
    Lineage registry selected.Row D where
  toFun := selected.inclusion

/-- Product rows retain lineage to their left source. -/
def productLeft
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (A : Type r) (B : Type s) : Lineage registry (A × B) A where
  toFun := Prod.fst

/-- Product rows retain lineage to their right source. -/
def productRight
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    (A : Type r) (B : Type s) : Lineage registry (A × B) B where
  toFun := Prod.snd

/-- Replicated copy rows retain their unique source-row lineage. -/
def copies
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {X : Type r} (count : X → Nat) : Lineage registry (Copies count) X where
  toFun := Copies.source

/-- Successful registered localization freezes point-to-cell lineage for later gather/write-back. -/
def located
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) :
    Lineage registry J (CellRef schema.sites site) where
  toFun := rows.destination

/-- Registered interpolation edges retain their source-position row lineage. -/
def interpolationSource
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions) :
    Lineage registry rows.span.Edge J where
  toFun := rows.span.source

/-- Registered interpolation edges retain their destination-cell lineage. -/
def interpolationCell
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions) :
    Lineage registry rows.span.Edge (CellRef schema.sites habitat) where
  toFun := rows.span.cell

/-- Gather a field only through certified semantic lineage. -/
def gather
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {J : Type r} {I : Type s} (lineage : Lineage registry J I)
    {V : Type t} (field : Field I V) : Field J V :=
  Field.reindex lineage field

@[simp] theorem refl_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {D : Type r} (site : D) :
    refl registry D site = site :=
  rfl

@[simp] theorem comp_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {I : Type r} {J : Type s} {K : Type t}
    (outer : Lineage registry J I) (inner : Lineage registry K J) (row : K) :
    comp outer inner row = outer (inner row) :=
  rfl

@[simp] theorem gather_refl
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {D : Type r} {V : Type s} (field : Field D V) :
    gather (refl registry D) field = field :=
  rfl

@[simp] theorem gather_comp
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {I : Type r} {J : Type s} {K : Type t}
    (outer : Lineage registry J I) (inner : Lineage registry K J)
    {V : Type u} (field : Field I V) :
    gather (comp outer inner) field = gather inner (gather outer field) :=
  rfl
/-- Gather cell values to interpolation edges only through the registered cell leg. -/
def interpolationGather
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (field : Field (CellRef schema.sites habitat) V) :
    Field rows.span.Edge V :=
  gather (interpolationCell rows) field

/-- Scale registered edge gathers by their certified interpolation coefficients. -/
def interpolationWeightedGather
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (values : WeightedValueLaw weights V)
    (field : Field (CellRef schema.sites habitat) V) :
    Field rows.span.Edge V :=
  fun edge => values.scale (rows.span.coefficient edge)
    (interpolationGather rows field edge)

/-- Registered interpolation reduces only gathered edge values back to source rows. -/
def interpolationSample
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (values : WeightedValueLaw weights V)
    (field : Field (CellRef schema.sites habitat) V) : Field J V :=
  rows.span.reduceFiber values (interpolationWeightedGather rows values field)


theorem gather_comp_operator
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {I : Type r} {J : Type s} {K : Type t}
    (outer : Lineage registry J I) (inner : Lineage registry K J)
    {V : Type u} :
    (fun field : Field I V => gather (comp outer inner) field) =
      (fun field : Field I V => gather inner (gather outer field)) :=
  rfl

@[simp] theorem registered_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {source target : schema.sites.Id}
    (token : registry.LineageToken source target)
    (site : CellRef schema.sites source) :
    registered registry token site = registry.lineageMap token site :=
  rfl

@[simp] theorem selection_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {D : Type r} (selected : Selection D) (row : selected.Row) :
    selection registry selected row = selected.inclusion row :=
  rfl

@[simp] theorem copies_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    (registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames)
    {X : Type r} (count : X → Nat) (copy : Copies count) :
    copies registry count copy = Copies.source copy :=
  rfl

@[simp] theorem located_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {site : schema.sites.Id} {frame : frames.Id}
    {spatial : registry.SituatedToken site frame}
    {J : Type r} {positions : Field J (Point frames frame)}
    (rows : LocatedRows registry spatial positions) (row : J) :
    located rows row = rows.destination row :=
  rfl

@[simp] theorem interpolationSource_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    (edge : rows.span.Edge) :
    interpolationSource rows edge = rows.span.source edge :=
  rfl

@[simp] theorem interpolationCell_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    (edge : rows.span.Edge) :
    interpolationCell rows edge = rows.span.cell edge :=
  rfl

@[simp] theorem interpolationGather_apply
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (field : Field (CellRef schema.sites habitat) V)
    (edge : rows.span.Edge) :
    interpolationGather rows field edge = field (rows.span.cell edge) :=
  rfl

theorem interpolationGather_eq_spanGather
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (field : Field (CellRef schema.sites habitat) V) :
    interpolationGather rows field = rows.span.gather field :=
  rfl

theorem interpolationSample_eq_spanSample
    {schema : Schema.{u, v, w, x}} {frames : FrameSchema.{y, z, p}}
    {registry : SpatialRegistry.{u, v, w, x, y, z, p, q} schema frames}
    {K : Type r} {weights : WeightLaw K}
    {authority : SpatialInterpolationRegistry registry K weights}
    {habitat : schema.sites.Id} {frame : frames.Id}
    {token : authority.Token habitat frame}
    {J : Type s} {positions : Field J (Point frames frame)}
    (rows : RegisteredInterpolationRows registry authority token positions)
    {V : Type t} (values : WeightedValueLaw weights V)
    (field : Field (CellRef schema.sites habitat) V) :
    interpolationSample rows values field = rows.span.sample values field :=
  rfl

end Lineage

end Ano
