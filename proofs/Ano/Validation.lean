import Ano.Assignment
import Ano.World

namespace Ano.Effects

universe u v w y

inductive AssignmentValidationError (J : Type u) (D : Type v) where
  | invalidDestination (row : J)
  | collision (destination : D)

inductive ValidatedRows {J : Type u} {D : Type v} (raw : J → Option D) :
    List J → List D → Type (max u v) where
  | nil : ValidatedRows raw [] []
  | cons {row : J} {rows : List J} {destination : D} {destinations : List D}
      (valid : raw row = some destination)
      (tail : ValidatedRows raw rows destinations)
      (fresh : destination ∉ destinations) :
      ValidatedRows raw (row :: rows) (destination :: destinations)

namespace ValidatedRows

theorem length_eq {J : Type u} {D : Type v} {raw : J → Option D}
    {rows : List J} {destinations : List D}
    (checked : ValidatedRows raw rows destinations) :
    destinations.length = rows.length := by
  induction checked with
  | nil => rfl
  | cons valid tail fresh inductionHypothesis => simp [inductionHypothesis]

theorem destinations_nodup {J : Type u} {D : Type v} {raw : J → Option D}
    {rows : List J} {destinations : List D}
    (checked : ValidatedRows raw rows destinations) :
    destinations.Nodup := by
  induction checked with
  | nil => exact .nil
  | cons valid tail fresh inductionHypothesis =>
      refine .cons ?_ inductionHypothesis
      intro destination member same
      exact fresh (same ▸ member)

theorem all_valid {J : Type u} {D : Type v} {raw : J → Option D}
    {rows : List J} {destinations : List D}
    (checked : ValidatedRows raw rows destinations) :
    ∀ {row}, row ∈ rows → ∃ destination, raw row = some destination := by
  intro row member
  induction checked with
  | nil => cases member
  | @cons head rows destination destinations valid tail fresh inductionHypothesis =>
      cases member with
      | head => exact ⟨destination, valid⟩
      | tail _ member => exact inductionHypothesis member

theorem value_mem {J : Type u} {D : Type v} {raw : J → Option D}
    {rows : List J} {destinations : List D}
    (checked : ValidatedRows raw rows destinations)
    {row : J} {destination : D} (member : row ∈ rows)
    (valid : raw row = some destination) :
    destination ∈ destinations := by
  induction checked generalizing row destination with
  | nil => cases member
  | @cons head rows headDestination destinations headValid tail fresh inductionHypothesis =>
      cases member with
      | head =>
          have same : headDestination = destination :=
            Option.some.inj (headValid.symm.trans valid)
          subst destination
          exact .head _
      | tail _ member => exact .tail _ (inductionHypothesis member valid)

theorem collision_free {J : Type u} {D : Type v} {raw : J → Option D}
    {rows : List J} {destinations : List D}
    (checked : ValidatedRows raw rows destinations)
    {left right : J} {destination : D}
    (leftMember : left ∈ rows) (rightMember : right ∈ rows)
    (leftValid : raw left = some destination)
    (rightValid : raw right = some destination) :
    left = right := by
  induction checked generalizing left right destination with
  | nil => cases leftMember
  | @cons head rows headDestination destinations headValid tail fresh inductionHypothesis =>
      cases leftMember with
      | head =>
          cases rightMember with
          | head => rfl
          | tail _ rightMember =>
              have same : headDestination = destination :=
                Option.some.inj (headValid.symm.trans leftValid)
              subst destination
              exact False.elim (fresh (tail.value_mem rightMember rightValid))
      | tail _ leftMember =>
          cases rightMember with
          | head =>
              have same : headDestination = destination :=
                Option.some.inj (headValid.symm.trans rightValid)
              subst destination
              exact False.elim (fresh (tail.value_mem leftMember leftValid))
          | tail _ rightMember =>
              exact inductionHypothesis leftMember rightMember leftValid rightValid

end ValidatedRows

def validateRows {J : Type u} {D : Type v} [DecidableEq D]
    (raw : J → Option D) :
    (rows : List J) →
      Except (AssignmentValidationError J D)
        (Sigma fun destinations => ValidatedRows raw rows destinations)
  | [] => .ok ⟨[], .nil⟩
  | row :: rows =>
      match h : raw row with
      | none => .error (.invalidDestination row)
      | some destination =>
          match validateRows raw rows with
          | .error error => .error error
          | .ok ⟨destinations, tail⟩ =>
              if fresh : destination ∈ destinations then
                .error (.collision destination)
              else
                .ok ⟨destination :: destinations, .cons h tail fresh⟩

structure RawAssignment (J : Type u) (D : Type v) where
  schedule : AssignmentRows J
  destination : J → Option D

structure CheckedAssignment {J : Type u} {D : Type v}
    (raw : RawAssignment J D) where
  destinations : List D
  checked : ValidatedRows raw.destination raw.schedule.rows destinations

def validateAssignment {J : Type u} {D : Type v} [DecidableEq D]
    (raw : RawAssignment J D) :
    Except (AssignmentValidationError J D) (CheckedAssignment raw) :=
  match validateRows raw.destination raw.schedule.rows with
  | .error error => .error error
  | .ok ⟨destinations, checked⟩ => .ok ⟨destinations, checked⟩

namespace CheckedAssignment

noncomputable def destination {J : Type u} {D : Type v}
    {raw : RawAssignment J D} (checked : CheckedAssignment raw) : J → D :=
  fun row => Classical.choose
    (checked.checked.all_valid (raw.schedule.complete row))

theorem destination_spec {J : Type u} {D : Type v}
    {raw : RawAssignment J D} (checked : CheckedAssignment raw) (row : J) :
    raw.destination row = some (checked.destination row) :=
  Classical.choose_spec
    (checked.checked.all_valid (raw.schedule.complete row))

theorem destination_injective {J : Type u} {D : Type v}
    {raw : RawAssignment J D} (checked : CheckedAssignment raw) :
    Function.Injective checked.destination := by
  intro left right same
  apply checked.checked.collision_free
    (raw.schedule.complete left) (raw.schedule.complete right)
    (checked.destination_spec left)
  rw [same]
  exact checked.destination_spec right

noncomputable def execute {J : Type u} {D : Type v} [DecidableEq D]
    {raw : RawAssignment J D} (checked : CheckedAssignment raw)
    {V : Type w} (base : Field D V) (values : Field J V) : Field D V :=
  executeCertifiedAssignment raw.schedule checked.destination
    checked.destination_injective base values

end CheckedAssignment

theorem invalid_destination_rejected {J : Type u} {D : Type v} [DecidableEq D]
    (raw : RawAssignment J D) {row : J}
    (invalid : raw.destination row = none) :
    ∃ error, validateAssignment raw = .error error := by
  cases result : validateAssignment raw with
  | error error => exact ⟨error, rfl⟩
  | ok checked =>
      obtain ⟨destination, valid⟩ :=
        checked.checked.all_valid (raw.schedule.complete row)
      simp [invalid] at valid

theorem collision_rejected {J : Type u} {D : Type v} [DecidableEq D]
    (raw : RawAssignment J D) {left right : J} {destination : D}
    (distinct : left ≠ right)
    (leftValid : raw.destination left = some destination)
    (rightValid : raw.destination right = some destination) :
    ∃ error, validateAssignment raw = .error error := by
  cases result : validateAssignment raw with
  | error error => exact ⟨error, rfl⟩
  | ok checked =>
      exact False.elim (distinct (checked.checked.collision_free
        (raw.schedule.complete left) (raw.schedule.complete right)
        leftValid rightValid))

def validatedPlanner {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output) :
    Planner schema Output :=
  fun world =>
    match validateAssignment (raw world) with
    | .error error => .refuse (renderError error)
    | .ok checked => accepted world checked

theorem validation_rejection_refuses {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output)
    (world : World schema) (error : AssignmentValidationError J D)
    (rejected : validateAssignment (raw world) = .error error) :
    validatedPlanner raw renderError accepted world =
      .refuse (renderError error) := by
  simp [validatedPlanner, rejected]

theorem validation_rejection_atomic {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output)
    (world : World schema) (error : AssignmentValidationError J D)
    (rejected : validateAssignment (raw world) = .error error) :
    perform (validatedPlanner raw renderError accepted) world =
      .refused world (renderError error) :=
  perform_refusal_atomic (validatedPlanner raw renderError accepted) world
    (renderError error)
    (validation_rejection_refuses raw renderError accepted world error rejected)

theorem invalid_destination_atomic {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output)
    (world : World schema) {row : J}
    (invalid : (raw world).destination row = none) :
    ∃ error, perform (validatedPlanner raw renderError accepted) world =
      .refused world (renderError error) := by
  obtain ⟨error, rejected⟩ :=
    invalid_destination_rejected (raw world) invalid
  exact ⟨error, validation_rejection_atomic raw renderError accepted
    world error rejected⟩

theorem collision_atomic {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output)
    (world : World schema) {left right : J} {destination : D}
    (distinct : left ≠ right)
    (leftValid : (raw world).destination left = some destination)
    (rightValid : (raw world).destination right = some destination) :
    ∃ error, perform (validatedPlanner raw renderError accepted) world =
      .refused world (renderError error) := by
  obtain ⟨error, rejected⟩ :=
    collision_rejected (raw world) distinct leftValid rightValid
  exact ⟨error, validation_rejection_atomic raw renderError accepted
    world error rejected⟩
theorem validated_ticks_wellFormed {schema : Schema} {Output : Type y}
    {J : Type u} {D : Type v} [DecidableEq D]
    (raw : World schema → RawAssignment J D)
    (renderError : AssignmentValidationError J D → String)
    (accepted : (world : World schema) →
      CheckedAssignment (raw world) → Plan schema Output)
    (steps : Nat) {world after : World schema}
    (hworld : WellFormed world)
    (ran : ticks (validatedPlanner raw renderError accepted) steps world =
      some after) :
    WellFormed after :=
  ticks_wellFormed (validatedPlanner raw renderError accepted)
    steps hworld ran

end Ano.Effects
