import Ano.Spawn

namespace Ano.Allocation

/-- Existing identities embed into the left side of the enlarged live population. -/
def oldKey {oldCount added : Nat} : Fin oldCount → Fin (oldCount + added) :=
  Fin.castAdd added

/-- Spawned identities occupy the fresh right side of the enlarged live population. -/
def freshKey (oldCount : Nat) {added : Nat} : Fin added → Fin (oldCount + added) :=
  Fin.natAdd oldCount

theorem oldKey_injective {oldCount added : Nat} :
    Function.Injective (oldKey (oldCount := oldCount) (added := added)) := by
  intro left right h
  apply Fin.ext
  exact congrArg (fun i : Fin (oldCount + added) => i.val) h

theorem freshKey_injective (oldCount : Nat) {added : Nat} :
    Function.Injective (freshKey oldCount (added := added)) := by
  intro left right h
  apply Fin.ext
  exact Nat.add_left_cancel
    (congrArg (fun i : Fin (oldCount + added) => i.val) h)

theorem freshKey_not_old {oldCount added : Nat}
    (old : Fin oldCount) (fresh : Fin added) :
    oldKey (added := added) old ≠ freshKey oldCount fresh := by
  intro h
  have hValues : old.val = oldCount + fresh.val :=
    congrArg (fun i : Fin (oldCount + added) => i.val) h
  have hLess : old.val < oldCount := old.isLt
  have hAtLeast : oldCount ≤ oldCount + fresh.val := Nat.le_add_right oldCount fresh.val
  rw [hValues] at hLess
  exact (Nat.not_lt_of_ge hAtLeast) hLess

/-- Copy rows receive distinct fresh identities by their duplicate-free enumeration slots. -/
def allocateCopySlots (oldCount : Nat) {n : Nat} {X : Type}
    (layout : Layout n X) (count : X → Nat) :
    Fin (Copies.enumerate layout count).length →
      Fin (oldCount + (Copies.enumerate layout count).length) :=
  freshKey oldCount

theorem allocateCopySlots_injective (oldCount : Nat) {n : Nat} {X : Type}
    (layout : Layout n X) (count : X → Nat) :
    Function.Injective (allocateCopySlots oldCount layout count) :=
  freshKey_injective oldCount

theorem allocatedCopySlot_not_old (oldCount : Nat) {n : Nat} {X : Type}
    (layout : Layout n X) (count : X → Nat) (old : Fin oldCount)
    (slot : Fin (Copies.enumerate layout count).length) :
    oldKey (added := (Copies.enumerate layout count).length) old ≠
      allocateCopySlots oldCount layout count slot :=
  freshKey_not_old old slot

end Ano.Allocation
