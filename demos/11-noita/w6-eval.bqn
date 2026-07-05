# Quotation (w6): eval of a quoted statement equals the statement written directly.
# Ano: SparkBolt , Damage += 10
#      eval "SparkBolt , Damage += 10"
# Semantics: the quotation is a literal, so it resolves at compile time — a splice, not an interpreter — and the spliced statement's footprint stays visible to §11's static conflict check. That is the constrained ⍎: reflection over program text priced the same as program text. The dynamic form (a string built at runtime) has an opaque footprint and waits on the interpreter; the analyzable middle — templates with holes, the koto nominalizer — stays open.
# Invariant: direct and quoted agree cell for cell; the bomb bystander is untouched twice.

d ← 3‿3‿8 ⋄ sb ← 1‿1‿0
Cast ← {𝕩 + 10×sb}

direct ← Cast d
q ← •BQN "{𝕩 + 10×1‿1‿0}"                # ⍎ of the same text, closed over nothing
! direct ≡ q {𝕎𝕩} d
! 23‿23‿8 ≡ Cast q {𝕎𝕩} d                # the two barriers stack
