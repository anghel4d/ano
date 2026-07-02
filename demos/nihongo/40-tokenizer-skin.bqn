# Ex40 — Japanese tokenizer skin: two readers, one token stream, one statement.
# ano: 北 と 両手 六十 より 、 金 に 千 たす
# ano: あの Nord & TwoHanded > 60 が Gold += 1000
# ano: Nord & TwoHanded > 60 , Gold += 1000
# Semantics: each surface is a lexer table over abstract token kinds; the Japanese reader drops the fused に target marker and re-roots postfix operators, then both surfaces run the same statement: add 1000 Gold under the selection mask.
# Invariant: the normalized Japanese kind stream equals the ASCII kind stream, and Gold's post-state equals the hand-computed column.

# ASCII reader output.
ascii ← ⟨"IDENT:Nord","AND","IDENT:TwoHanded","GT","NUM:60","COMMA","IDENT:Gold","ADDASSIGN","NUM:1000"⟩

# Japanese reader: a lexer table over the closed registry vocabulary, then the postfix rule.
sentence ← ⟨"北","と","両手","六十","より","、","金","に","千","たす"⟩
vocab    ← ⟨"北","と","両手","六十","より","、","金","に","千","たす"⟩
kind     ← ⟨"IDENT:Nord","AND","IDENT:TwoHanded","NUM:60","GT","COMMA","IDENT:Gold","TGT","NUM:1000","ADDASSIGN"⟩
lexed ← (vocab⊐sentence)⊏kind
noTgt ← lexed/˜¬"TGT"⊸≡¨lexed     # に fuses into たす's ADDASSIGN
jp ← 0‿1‿2‿4‿3‿5‿6‿8‿7⊏noTgt      # re-root each postfix operator before its operand
"ex40: one token stream modulo reader surface"!(ascii≡jp)

# The statement, on columns. 6 entities.
nord      ← 1‿1‿0‿1‿1‿0
twoHanded ← 70‿55‿90‿61‿60‿80
gold      ← 100‿100‿100‿100‿100‿100
gold +↩ 1000×nord∧twoHanded>60
"ex40: post-state Gold"!(1100‿100‿100‿1100‿100‿100≡gold)
