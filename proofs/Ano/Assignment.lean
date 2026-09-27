import Ano.Effects

namespace Ano.Effects

universe u v w

/-- A certified physical schedule contains each logical query row exactly once. -/
structure AssignmentRows (J : Type u) where
  rows : List J
  nodup : rows.Nodup
  complete : ∀ row, row ∈ rows

def AssignmentRows.rowAt {J : Type u} (schedule : AssignmentRows J) :
    Fin schedule.rows.length → J :=
  fun i => schedule.rows[i]

theorem AssignmentRows.rowAt_injective {J : Type u} (schedule : AssignmentRows J) :
    Function.Injective schedule.rowAt := by
  intro left right h
  apply Fin.ext
  exact (List.getElem_inj schedule.nodup).mp h

def AssignmentRows.destinationAt {J : Type u} {D : Type v}
    (schedule : AssignmentRows J) (dst : J → D) :
    Fin schedule.rows.length → D :=
  dst ∘ schedule.rowAt

theorem AssignmentRows.destinationAt_injective {J : Type u} {D : Type v}
    (schedule : AssignmentRows J) (dst : J → D)
    (hInjective : Function.Injective dst) :
    Function.Injective (schedule.destinationAt dst) := by
  intro left right h
  exact schedule.rowAt_injective (hInjective h)

/-- With a duplicate-free row schedule and injective destination, each physical destination has at most one write occurrence. -/
theorem assignment_occurrence_fiber_subsingleton {J : Type u} {D : Type v}
    (schedule : AssignmentRows J) (dst : J → D)
    (hInjective : Function.Injective dst) (d : D) :
    Subsingleton (AssignmentFiber (schedule.destinationAt dst) d) :=
  assignment_fiber_subsingleton (schedule.destinationAt dst)
    (schedule.destinationAt_injective dst hInjective) d

def executeCertifiedAssignment {J : Type u} {D : Type v} [DecidableEq D]
    (schedule : AssignmentRows J) (dst : J → D) {V : Type w}
    (_hInjective : Function.Injective dst)
    (base : Field D V) (values : Field J V) : Field D V :=
  executeAssignment dst base values schedule.rows

end Ano.Effects
