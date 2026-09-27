import Ano.Field

namespace Ano

universe u v w

namespace Scatter

/-- Semantic assignment chooses the unique row targeting a site, independently of traversal order. -/
noncomputable def assign {J : Type u} {D : Type v} (dst : J → D) {V : Type w}
    (base : Field D V) (values : Field J V) : Field D V := by
  classical
  exact fun d => if h : ∃ j, dst j = d then values (Classical.choose h) else base d

theorem assign_hit {J : Type u} {D : Type v} (dst : J → D)
    (hInjective : Function.Injective dst) {V : Type w}
    (base : Field D V) (values : Field J V) (j : J) :
    assign dst base values (dst j) = values j := by
  classical
  have h : ∃ k, dst k = dst j := ⟨j, rfl⟩
  rw [assign, dif_pos h]
  exact congrArg values (hInjective (Classical.choose_spec h))

theorem assign_miss {J : Type u} {D : Type v} (dst : J → D)
    {V : Type w} (base : Field D V) (values : Field J V) (d : D)
    (hMiss : ¬ ∃ j, dst j = d) :
    assign dst base values d = base d := by
  classical
  simp [assign, hMiss]

def Satisfies {J : Type u} {D : Type v} (dst : J → D) {V : Type w}
    (base : Field D V) (values : Field J V) (out : Field D V) : Prop :=
  (∀ j, out (dst j) = values j) ∧
  (∀ d, (¬ ∃ j, dst j = d) → out d = base d)

theorem assign_satisfies {J : Type u} {D : Type v} (dst : J → D)
    (hInjective : Function.Injective dst) {V : Type w}
    (base : Field D V) (values : Field J V) :
    Satisfies dst base values (assign dst base values) := by
  constructor
  · exact assign_hit dst hInjective base values
  · intro d hMiss
    exact assign_miss dst base values d hMiss

theorem satisfies_unique {J : Type u} {D : Type v} (dst : J → D)
    {V : Type w} (base : Field D V) (values : Field J V)
    {left right : Field D V}
    (hLeft : Satisfies dst base values left)
    (hRight : Satisfies dst base values right) :
    left = right := by
  funext d
  by_cases h : ∃ j, dst j = d
  · obtain ⟨j, rfl⟩ := h
    rw [hLeft.1 j, hRight.1 j]
  · rw [hLeft.2 d h, hRight.2 d h]

theorem injective_scatter_deterministic {J : Type u} {D : Type v}
    (dst : J → D) (hInjective : Function.Injective dst) {V : Type w}
    (base : Field D V) (values : Field J V) :
    ∃ out, Satisfies dst base values out ∧
      ∀ other, Satisfies dst base values other → other = out := by
  refine ⟨assign dst base values, assign_satisfies dst hInjective base values, ?_⟩
  intro other hOther
  exact satisfies_unique dst base values hOther
    (assign_satisfies dst hInjective base values)

theorem at_most_one_writer {J : Type u} {D : Type v} (dst : J → D)
    (hInjective : Function.Injective dst) (d : D) {j k : J}
    (hj : dst j = d) (hk : dst k = d) :
    j = k :=
  hInjective (hj.trans hk.symm)

end Scatter

namespace Effects

def assignmentStep {J : Type u} {D : Type v} [DecidableEq D]
    (dst : J → D) {V : Type w} (values : Field J V)
    (d : D) (current : V) (row : J) : V :=
  if dst row = d then values row else current

def executeAssignment {J : Type u} {D : Type v} [DecidableEq D]
    (dst : J → D) {V : Type w} (base : Field D V) (values : Field J V)
    (rows : List J) : Field D V :=
  fun d => rows.foldl (assignmentStep dst values d) (base d)

theorem assignmentStep_commutes {J : Type u} {D : Type v} [DecidableEq D]
    (dst : J → D) (hInjective : Function.Injective dst)
    {V : Type w} (values : Field J V) (d : D) (current : V) (left right : J) :
    assignmentStep dst values d (assignmentStep dst values d current left) right =
      assignmentStep dst values d (assignmentStep dst values d current right) left := by
  by_cases hLeft : dst left = d <;> by_cases hRight : dst right = d
  · have hRows : left = right := hInjective (hLeft.trans hRight.symm)
    subst right
    simp [assignmentStep, hLeft]
  · simp [assignmentStep, hLeft, hRight]
  · simp [assignmentStep, hLeft, hRight]
  · simp [assignmentStep, hLeft, hRight]

theorem injective_assignment_permutation_independent
    {J : Type u} {D : Type v} [DecidableEq D]
    (dst : J → D) (hInjective : Function.Injective dst)
    {V : Type w} (base : Field D V) (values : Field J V)
    {left right : List J} (permutation : left.Perm right) :
    executeAssignment dst base values left = executeAssignment dst base values right := by
  funext d
  apply permutation.foldl_eq'
  intro row₁ _ row₂ _ current
  exact assignmentStep_commutes dst hInjective values d current row₁ row₂

def AssignmentFiber {J : Type u} {D : Type v} (dst : J → D) (d : D) :=
  {row : J // dst row = d}

theorem assignment_fiber_subsingleton {J : Type u} {D : Type v}
    (dst : J → D) (hInjective : Function.Injective dst) (d : D) :
    Subsingleton (AssignmentFiber dst d) := by
  constructor
  intro left right
  apply Subtype.ext
  exact hInjective (left.property.trans right.property.symm)

/-- A merge law is trusted only with the commutative-monoid equations required by unordered fibers. -/
structure CommMonoidLaw (V : Type w) where
  op : V → V → V
  identity : V
  associative : ∀ a b c, op (op a b) c = op a (op b c)
  commutative : ∀ a b, op a b = op b a
  leftIdentity : ∀ a, op identity a = a
  rightIdentity : ∀ a, op a identity = a

def mergeStep {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (values : Field J V)
    (d : D) (current : V) (row : J) : V :=
  if dst row = d then law.op current (values row) else current

theorem mergeStep_commutes {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (values : Field J V)
    (d : D) (current : V) (left right : J) :
    mergeStep law dst values d (mergeStep law dst values d current left) right =
      mergeStep law dst values d (mergeStep law dst values d current right) left := by
  by_cases hLeft : dst left = d <;> by_cases hRight : dst right = d
  · simp only [mergeStep, if_pos hLeft, if_pos hRight]
    calc
      law.op (law.op current (values left)) (values right) =
          law.op current (law.op (values left) (values right)) :=
        law.associative current (values left) (values right)
      _ = law.op current (law.op (values right) (values left)) :=
        congrArg (law.op current) (law.commutative (values left) (values right))
      _ = law.op (law.op current (values right)) (values left) :=
        (law.associative current (values right) (values left)).symm
  · simp [mergeStep, hLeft, hRight]
  · simp [mergeStep, hLeft, hRight]
  · simp [mergeStep, hLeft, hRight]

def reduceMergeFiber {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (values : Field J V)
    (rows : List J) : Field D V :=
  fun d => rows.foldl (mergeStep law dst values d) law.identity

def executeMerge {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (base : Field D V)
    (values : Field J V) (rows : List J) : Field D V :=
  fun d => rows.foldl (mergeStep law dst values d) (base d)

theorem commutative_merge_permutation_independent
    {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (values : Field J V)
    {left right : List J} (permutation : left.Perm right) :
    reduceMergeFiber law dst values left = reduceMergeFiber law dst values right := by
  funext d
  apply permutation.foldl_eq'
  intro row₁ _ row₂ _ current
  exact mergeStep_commutes law dst values d current row₁ row₂

theorem executeMerge_permutation_independent
    {J : Type u} {D : Type v} [DecidableEq D]
    (law : CommMonoidLaw V) (dst : J → D) (base : Field D V)
    (values : Field J V) {left right : List J} (permutation : left.Perm right) :
    executeMerge law dst base values left = executeMerge law dst base values right := by
  funext d
  apply permutation.foldl_eq'
  intro row₁ _ row₂ _ current
  exact mergeStep_commutes law dst values d current row₁ row₂

end Effects

end Ano
