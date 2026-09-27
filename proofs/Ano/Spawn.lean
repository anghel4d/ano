import Ano.Field

namespace Ano

universe u v

/-- Copies produced by one source row and its requested multiplicity. -/
abbrev Copies {X : Type u} (count : X → Nat) := Sigma fun x => Fin (count x)

namespace Copies

/-- Every copy retains its source row as lineage. -/
def source {X : Type u} {count : X → Nat} : Copies count → X :=
  Sigma.fst

@[simp] theorem source_mk {X : Type u} {count : X → Nat}
    (x : X) (number : Fin (count x)) : source (Sigma.mk x number) = x :=
  rfl

/-- Source values reach copies only by gathering through source lineage. -/
def gather {X : Type u} {count : X → Nat} {V : Type v}
    (field : Field X V) : Field (Copies count) V :=
  Field.reindex source field

private theorem nodup_map_of_injective {A : Type u} {B : Type v}
    {f : A → B} (hInjective : Function.Injective f) :
    ∀ {rows : List A}, rows.Nodup → (rows.map f).Nodup := by
  intro rows hRows
  induction rows with
  | nil => simp
  | cons row rows ih =>
      rw [List.nodup_cons] at hRows
      change (f row :: rows.map f).Nodup
      rw [List.nodup_cons]
      constructor
      · intro hMem
        rw [List.mem_map] at hMem
        obtain ⟨other, hOther, hEq⟩ := hMem
        exact hRows.1 (hInjective hEq ▸ hOther)
      · exact ih hRows.2

private theorem finRange_nodup : ∀ n, (List.finRange n).Nodup
  | 0 => by simp
  | n + 1 => by
      rw [List.finRange_succ, List.nodup_cons]
      constructor
      · intro hMem
        rw [List.mem_map] at hMem
        obtain ⟨i, _, hEq⟩ := hMem
        have hVal := congrArg Fin.val hEq
        simp at hVal
      · apply nodup_map_of_injective
        · intro left right hEq
          apply Fin.ext
          exact Nat.succ.inj (congrArg Fin.val hEq)
        · exact finRange_nodup n

/-- The copy rows of one source, in increasing copy-number order. -/
def forSource {X : Type u} (count : X → Nat) (x : X) : List (Copies count) :=
  (List.finRange (count x)).map fun number => ⟨x, number⟩

theorem mem_forSource {X : Type u} (count : X → Nat) (x : X)
    (number : Fin (count x)) :
    (Sigma.mk x number : Copies count) ∈ forSource count x := by
  rw [forSource, List.mem_map]
  exact ⟨number, List.mem_finRange number, rfl⟩

theorem source_eq_of_mem_forSource {X : Type u} {count : X → Nat}
    {x : X} {copy : Copies count} (h : copy ∈ forSource count x) :
    source copy = x := by
  rw [forSource, List.mem_map] at h
  obtain ⟨number, _, hCopy⟩ := h
  subst copy
  rfl

theorem forSource_nodup {X : Type u} (count : X → Nat) (x : X) :
    (forSource count x).Nodup := by
  apply nodup_map_of_injective
  · intro left right h
    exact Fin.ext (congrArg (fun c : Copies count => c.snd.val) h)
  · exact finRange_nodup (count x)

theorem length_forSource {X : Type u} (count : X → Nat) (x : X) :
    (forSource count x).length = count x := by
  simp [forSource]

private theorem nodup_flatMap_of_disjoint {A : Type u} {B : Type v}
    {rows : List A} {f : A → List B}
    (hRows : rows.Nodup)
    (hEach : ∀ row, row ∈ rows → (f row).Nodup)
    (hApart : ∀ left, left ∈ rows → ∀ right, right ∈ rows → left ≠ right →
      ∀ x, x ∈ f left → ∀ y, y ∈ f right → x ≠ y) :
    (rows.flatMap f).Nodup := by
  induction rows with
  | nil => simp
  | cons row rows ih =>
      rw [List.nodup_cons] at hRows
      rw [List.flatMap_cons, List.nodup_append]
      refine ⟨hEach row (by simp), ?_, ?_⟩
      · apply ih hRows.2
        · intro other hOther
          exact hEach other (by simp [hOther])
        · intro left hLeft right hRight hNe x hx y hy
          exact hApart left (by simp [hLeft]) right (by simp [hRight]) hNe x hx y hy
      · intro x hx y hy
        rw [List.mem_flatMap] at hy
        obtain ⟨other, hOther, hy⟩ := hy
        exact hApart row (by simp) other (by simp [hOther])
          (fun h => hRows.1 (h ▸ hOther)) x hx y hy

/-- Layout-order enumeration of every dependent copy row. -/
def enumerate {n : Nat} {X : Type u} (layout : Layout n X) (count : X → Nat) :
    List (Copies count) :=
  (List.finRange n).flatMap fun slot => forSource count (layout slot)

theorem mem_enumerate {n : Nat} {X : Type u} (layout : Layout n X)
    (count : X → Nat) (copy : Copies count) :
    copy ∈ enumerate layout count := by
  rw [enumerate, List.mem_flatMap]
  refine ⟨layout.invFun copy.fst, List.mem_finRange _, ?_⟩
  have hSite : layout (layout.invFun copy.fst) = copy.fst := layout.rightInv copy.fst
  simpa only [hSite] using (mem_forSource count copy.fst copy.snd)

theorem enumerate_nodup {n : Nat} {X : Type u} (layout : Layout n X)
    (count : X → Nat) : (enumerate layout count).Nodup := by
  apply nodup_flatMap_of_disjoint (finRange_nodup n)
  · intro slot _
    exact forSource_nodup count (layout slot)
  · intro left _ right _ hNe copyLeft hLeft copyRight hRight hEq
    have hsLeft := source_eq_of_mem_forSource hLeft
    have hsRight := source_eq_of_mem_forSource hRight
    have hsCopies : source copyLeft = source copyRight := congrArg source hEq
    have hSites : layout left = layout right := hsLeft.symm.trans (hsCopies.trans hsRight)
    exact hNe (layout.injective hSites)

/-- Sum of copy counts over the semantic sources in physical layout order. -/
def total {n : Nat} {X : Type u} (layout : Layout n X) (count : X → Nat) : Nat :=
  ((List.finRange n).map fun slot => count (layout slot)).sum

theorem length_enumerate {n : Nat} {X : Type u} (layout : Layout n X)
    (count : X → Nat) :
    (enumerate layout count).length = total layout count := by
  simp [enumerate, total, length_forSource]

theorem source_unique {X : Type u} {count : X → Nat} (copy : Copies count) :
    Exists (fun x => source copy = x ∧ ∀ y, source copy = y → y = x) := by
  refine ⟨source copy, rfl, ?_⟩
  intro y h
  exact h.symm

@[simp] theorem gather_apply {X : Type u} {count : X → Nat} {V : Type v}
    (field : Field X V) (copy : Copies count) :
    gather field copy = field (source copy) :=
  rfl

@[simp] theorem gather_mk {X : Type u} {count : X → Nat} {V : Type v}
    (field : Field X V) (x : X) (number : Fin (count x)) :
    gather field (Sigma.mk x number) = field x :=
  rfl

theorem gather_eq_of_source_eq {X : Type u} {count : X → Nat} {V : Type v}
    (field : Field X V) {left right : Copies count}
    (h : source left = source right) :
    gather field left = gather field right :=
  congrArg field h

end Copies

end Ano
