import Ano.SpatialPlan

namespace Ano.SpatialNegativeWitness

/-- Equal raw carriers do not erase nominal habitat identity. -/
inductive HabitatId where
  | ground
  | mars

def sites : SiteSchema where
  Id := HabitatId
  Carrier := fun _ => Fin 4

abbrev Ground := CellRef sites .ground
abbrev Mars := CellRef sites .mars

/-- Equal raw coordinate carriers do not erase nominal frame identity. -/
inductive FrameId where
  | playerPlane
  | world3
  | mars3

def frames : FrameSchema where
  Id := FrameId
  PointCarrier := fun _ => Nat
  VectorCarrier := fun _ => Nat

def schema : Schema where
  sites := sites
  FieldId := Empty
  fieldSite := fun field => nomatch field
  Value := fun field => nomatch field
  valid := fun field => nomatch field
  globalValid := fun _ _ => True
  spawnPreserves := by
    intro _ _ _ _ _
    trivial

/-- This witness registry grants no cross-habitat or spatial capabilities. -/
def registry : SpatialRegistry schema frames where
  PositionToken := fun _ _ => Empty
  positionView := fun token => nomatch token
  BoxToken := fun _ => Empty
  box := fun token => nomatch token
  LineageToken := fun _ _ => Empty
  lineageMap := fun token => nomatch token
  FrameMapToken := fun _ _ => Empty
  frameMap := fun token => nomatch token
  OffsetMapToken := fun _ _ => Empty
  offsetMap := fun token => nomatch token
  SituatedToken := fun _ _ => Empty
  situated := fun token => nomatch token

def groundLayout : Layout 4 Ground where
  toFun := fun slot => { index := slot }
  invFun := fun site => site.index
  leftInv := by intro site; rfl
  rightInv := by intro site; cases site; rfl

def pointwise {D : Type} {V : Type} (op : V → V → V)
    (left right : Field D V) : Field D V :=
  fun row => op (left row) (right row)

/--
error: Application type mismatch: The argument
  mars
has type
  Field Mars Nat
but is expected to have type
  Field Ground Nat
in the application
  pointwise Nat.add ground mars
-/
#guard_msgs (error, drop info) in
#check fun (ground : Field Ground Nat) (mars : Field Mars Nat) =>
  pointwise Nat.add ground mars -- Foreign equal-shaped habitats cannot combine pointwise.

def worldPointValue (point : Point frames .world3) : Nat :=
  point.coordinate

/--
error: Application type mismatch: The argument
  point
has type
  Point frames FrameId.mars3
but is expected to have type
  Point frames FrameId.world3
in the application
  worldPointValue point
-/
#guard_msgs (error, drop info) in
#check fun (point : Point frames .mars3) => worldPointValue point -- A point in another three-dimensional frame is still not a World3 point.

def translateWorld (point : Point frames .world3)
    (_offset : SpatialVector frames .world3) : Point frames .world3 :=
  point

/--
error: Application type mismatch: The argument
  offset
has type
  SpatialVector frames FrameId.playerPlane
but is expected to have type
  SpatialVector frames FrameId.world3
in the application
  translateWorld point offset
-/
#guard_msgs (error, drop info) in
#check fun (point : Point frames .world3)
    (offset : SpatialVector frames .playerPlane) =>
  translateWorld point offset -- A player-plane displacement cannot be added directly to a World3 point.

def readGround (field : Field Ground Nat) (site : Ground) : Nat :=
  field site

/--
error: Application type mismatch: The argument
  slot
has type
  Fin 4
but is expected to have type
  Ground
in the application
  readGround field slot
-/
#guard_msgs (error, drop info) in
#check fun (field : Field Ground Nat) (slot : Fin 4) => readGround field slot -- A physical row offset is not a nominal cell reference.

/--
error: Type mismatch
  layout
has type
  Layout 4 Ground
but is expected to have type
  Lineage registry (Fin 4) Ground
-/
#guard_msgs (error, drop info) in
#check fun (layout : Layout 4 Ground) =>
  (layout : Lineage registry (Fin 4) Ground) -- A physical layout has no coercion into registry-authorized semantic lineage.

def oneWorldLocator : Locator frames .world3 Unit where
  accepts := fun _ _ => True
  locate := fun _ => some ()
  sound := by intro _ _ _; trivial
  complete := by intro _point _accepted; exact ⟨(), rfl⟩
  functional := by intro _ left right _ _; cases left; cases right; rfl

/--
error: Application type mismatch: The argument
  point
has type
  Point frames FrameId.mars3
but is expected to have type
  Point frames FrameId.world3
in the application
  oneWorldLocator.locate point
-/
#guard_msgs (error, drop info) in
#check fun (point : Point frames .mars3) => oneWorldLocator.locate point -- A World3 locator cannot consume a Mars3 position.

/--
error: Type mismatch
  placement
has type
  Placement Ground frames FrameId.world3
but is expected to have type
  Locator frames FrameId.world3 Ground
-/
#guard_msgs (error, drop info) in
#check fun (placement : Placement Ground frames .world3) =>
  (placement : Locator frames .world3 Ground) -- A placement is a covariant point-valued map, not row lineage or an implicit locator.

end Ano.SpatialNegativeWitness
