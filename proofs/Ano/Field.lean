import Std

namespace Ano

universe u v w x

/-- A semantic field varies over a typed domain, independently of physical layout. -/
abbrev Field (D : Type u) (V : Type v) := D → V

/-- A dependency-free equivalence witness. -/
structure Iso (A : Type u) (B : Type v) where
  toFun : A → B
  invFun : B → A
  leftInv : ∀ a, invFun (toFun a) = a
  rightInv : ∀ b, toFun (invFun b) = b

namespace Iso

instance {A : Type u} {B : Type v} : CoeFun (Iso A B) (fun _ => A → B) :=
  ⟨Iso.toFun⟩

@[simp] theorem left_inv {A : Type u} {B : Type v} (e : Iso A B) (a : A) :
    e.invFun (e a) = a :=
  e.leftInv a

@[simp] theorem right_inv {A : Type u} {B : Type v} (e : Iso A B) (b : B) :
    e (e.invFun b) = b :=
  e.rightInv b

theorem injective {A : Type u} {B : Type v} (e : Iso A B) :
    Function.Injective e := by
  intro a b h
  calc
    a = e.invFun (e a) := (e.leftInv a).symm
    _ = e.invFun (e b) := congrArg e.invFun h
    _ = b := e.leftInv b

theorem surjective {A : Type u} {B : Type v} (e : Iso A B) :
    Function.Surjective e := by
  intro b
  exact ⟨e.invFun b, e.rightInv b⟩

def refl (A : Type u) : Iso A A where
  toFun := id
  invFun := id
  leftInv := fun _ => rfl
  rightInv := fun _ => rfl

def symm {A : Type u} {B : Type v} (e : Iso A B) : Iso B A where
  toFun := e.invFun
  invFun := e.toFun
  leftInv := e.rightInv
  rightInv := e.leftInv

def trans {A : Type u} {B : Type v} {C : Type w} (ab : Iso A B) (bc : Iso B C) :
    Iso A C where
  toFun := bc ∘ ab
  invFun := ab.invFun ∘ bc.invFun
  leftInv := by
    intro a
    simp
  rightInv := by
    intro c
    simp

end Iso

namespace Field

/-- Reindexing is contravariant gather along an output-to-input map. -/
def reindex {I : Type u} {J : Type v} (u : J → I) {V : Type w}
    (field : Field I V) : Field J V :=
  fun j => field (u j)

@[simp] theorem reindex_id {I : Type u} {V : Type v} (field : Field I V) :
    reindex id field = field :=
  rfl

@[simp] theorem reindex_comp {I : Type u} {J : Type v} {K : Type w}
    (u : J → I) (v : K → J) {V : Type x} (field : Field I V) :
    reindex (u ∘ v) field = reindex v (reindex u field) :=
  rfl

theorem reindex_comp_operator {I : Type u} {J : Type v} {K : Type w}
    (u : J → I) (v : K → J) {V : Type x} :
    (fun field : Field I V => reindex (u ∘ v) field) =
      (fun field : Field I V => reindex v (reindex u field)) :=
  rfl


end Field

/-- A physical layout is a bijection from dense storage slots to semantic sites. -/
abbrev Layout (n : Nat) (D : Type u) := Iso (Fin n) D

namespace Layout

def encode {n : Nat} {D : Type u} (layout : Layout n D) {V : Type v}
    (field : Field D V) : Fin n → V :=
  Field.reindex layout field

def decode {n : Nat} {D : Type u} (layout : Layout n D) {V : Type v}
    (buffer : Fin n → V) : Field D V :=
  Field.reindex layout.invFun buffer

@[simp] theorem decode_encode {n : Nat} {D : Type u} (layout : Layout n D)
    {V : Type v} (field : Field D V) :
    decode layout (encode layout field) = field := by
  funext d
  simp [decode, encode, Field.reindex]

@[simp] theorem encode_decode {n : Nat} {D : Type u} (layout : Layout n D)
    {V : Type v} (buffer : Fin n → V) :
    encode layout (decode layout buffer) = buffer := by
  funext i
  simp [decode, encode, Field.reindex]

/-- The new physical slot maps to the old slot containing the same semantic site. -/
def changeIndex {n : Nat} {D : Type u} (old new : Layout n D) : Iso (Fin n) (Fin n) :=
  new.trans old.symm

def change {n : Nat} {D : Type u} (old new : Layout n D) {V : Type v}
    (bufferInOld : Fin n → V) : Fin n → V :=
  Field.reindex (changeIndex old new) bufferInOld

@[simp] theorem change_encode {n : Nat} {D : Type u} (old new : Layout n D)
    {V : Type v} (field : Field D V) :
    change old new (encode old field) = encode new field := by
  funext i
  simp [change, changeIndex, Iso.trans, Iso.symm, encode, Field.reindex, Function.comp_def]

@[simp] theorem decode_change {n : Nat} {D : Type u} (old new : Layout n D)
    {V : Type v} (bufferInOld : Fin n → V) :
    decode new (change old new bufferInOld) = decode old bufferInOld := by
  funext d
  simp [decode, change, changeIndex, Iso.trans, Iso.symm, Field.reindex, Function.comp_def]

@[simp] theorem change_refl {n : Nat} {D : Type u} (layout : Layout n D)
    {V : Type v} (buffer : Fin n → V) :
    change layout layout buffer = buffer := by
  funext i
  simp [change, changeIndex, Iso.trans, Iso.symm, Field.reindex, Function.comp_def]

@[simp] theorem change_trans {n : Nat} {D : Type u} (a b c : Layout n D)
    {V : Type v} (buffer : Fin n → V) :
    change b c (change a b buffer) = change a c buffer := by
  funext i
  simp [change, changeIndex, Iso.trans, Iso.symm, Field.reindex, Function.comp_def]

/-- Equal physical cardinality makes an alignment possible only after both layouts are supplied. -/
def possibleAlignment {n : Nat} {A : Type u} {B : Type v}
    (left : Layout n A) (right : Layout n B) : Iso A B :=
  left.symm.trans right

theorem equalCardinality_possibleAlignment {n : Nat} {A : Type u} {B : Type v}
    (left : Nonempty (Layout n A)) (right : Nonempty (Layout n B)) :
    Nonempty (Iso A B) := by
  obtain ⟨left⟩ := left
  obtain ⟨right⟩ := right
  exact ⟨possibleAlignment left right⟩

end Layout

/-- A selection stores its predicate, so scatter cannot re-evaluate it against post-state. -/
structure Selection (D : Type u) where
  keep : D → Prop
  decideKeep : DecidablePred keep

namespace Selection

instance {D : Type u} (selection : Selection D) (d : D) : Decidable (selection.keep d) :=
  selection.decideKeep d

abbrev Row {D : Type u} (selection : Selection D) := {d : D // selection.keep d}

def inclusion {D : Type u} (selection : Selection D) : selection.Row → D :=
  Subtype.val

theorem inclusion_injective {D : Type u} (selection : Selection D) :
    Function.Injective selection.inclusion := by
  intro left right h
  exact Subtype.ext h

def freezeWhere {D : Type u} {V : Type v} (snapshot : Field D V)
    (test : D → V → Bool) : Selection D where
  keep d := test d (snapshot d) = true
  decideKeep _ := inferInstance
@[simp] theorem freezeWhere_keep {D : Type u} {V : Type v}
    (snapshot : Field D V) (test : D → V → Bool) (d : D) :
    (freezeWhere snapshot test).keep d = (test d (snapshot d) = true) :=
  rfl


def gather {D : Type u} (selection : Selection D) {V : Type v}
    (field : Field D V) : Field selection.Row V :=
  Field.reindex selection.inclusion field

def scatter {D : Type u} (selection : Selection D) {V : Type v}
    (base : Field D V) (selected : Field selection.Row V) : Field D V :=
  fun d => if h : selection.keep d then selected ⟨d, h⟩ else base d

@[simp] theorem scatter_selected {D : Type u} (selection : Selection D) {V : Type v}
    (base : Field D V) (selected : Field selection.Row V) (row : selection.Row) :
    scatter selection base selected (selection.inclusion row) = selected row := by
  simp [scatter, inclusion, row.property]

theorem scatter_unselected {D : Type u} (selection : Selection D) {V : Type v}
    (base : Field D V) (selected : Field selection.Row V) {d : D}
    (h : ¬ selection.keep d) :
    scatter selection base selected d = base d := by
  simp [scatter, h]

@[simp] theorem gather_scatter {D : Type u} (selection : Selection D) {V : Type v}
    (base : Field D V) (selected : Field selection.Row V) :
    gather selection (scatter selection base selected) = selected := by
  funext row
  exact scatter_selected selection base selected row

theorem gather_modify_scatter_selected {D : Type u} (selection : Selection D)
    {V : Type v} (base : Field D V)
    (modify : Field selection.Row V → Field selection.Row V) (row : selection.Row) :
    scatter selection base (modify (gather selection base)) (selection.inclusion row) =
      modify (gather selection base) row :=
  scatter_selected selection base _ row

theorem gather_modify_scatter_unselected {D : Type u} (selection : Selection D)
    {V : Type v} (base : Field D V)
    (modify : Field selection.Row V → Field selection.Row V) {d : D}
    (h : ¬ selection.keep d) :
    scatter selection base (modify (gather selection base)) d = base d :=
  scatter_unselected selection base _ h

theorem gather_modify_scatter_exact {D : Type u} (selection : Selection D)
    {V : Type v} (base : Field D V)
    (modify : Field selection.Row V → Field selection.Row V) (d : D) :
    scatter selection base (modify (gather selection base)) d =
      if h : selection.keep d then modify (gather selection base) ⟨d, h⟩ else base d :=
  rfl

theorem freezeWhere_scatter_selected {D : Type u} {V : Type v}
    (snapshot : Field D V) (test : D → V → Bool)
    (base : Field D V)
    (selected : Field (freezeWhere snapshot test).Row V)
    (d : D) (kept : test d (snapshot d) = true) :
    scatter (freezeWhere snapshot test) base selected d =
      selected ⟨d, kept⟩ := by
  simp [scatter, freezeWhere, kept]

end Selection

end Ano
