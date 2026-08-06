# Tour

Welcome to AnoLang. Ano is a small language for talking to a live game world by description — pick the things that match, then say what happens to them. The name is the Japanese distal demonstrative あの, "that one over there": you do not hold a handle and poke an object, you point at a situation and the host resolves who is in it. This tour walks the pieces one at a time. We start from the world the language is about, because once you see how that world is shaped, the sentences make sense on their own.


### Objects vs columns

People usually think of a video game as a pile of objects. A Nord is an instance of some class or actor type; he has fields for health and gold, methods to take damage or trade, and a place in a scene graph or entity list. To change every Nord who meets some condition, you walk that list, ask each object what it is, and mutate the ones that pass. The mental model is object-oriented: identity first, data hanging off it, behavior as methods on the thing.

There is another view, and it falls out almost naturally from what game engines started calling ECS. Instead of a Nord-the-object, the world is a columnar store: rows are entities, columns are components, and an archetype is just the set of columns that travel together. A Nord is not a class instance. He is a row whose `Race` cell says Nord — and who also has a Gold, a TwoHanded, an IsHostile — the same way a row in a SQL table is present under every column of that table, or a row in a kdb+ table sits under each of its vectors.

| Habit | View |
|---|---|
| Object-oriented | Nord = object. Loop. Ask. Mutate. |
| ECS / this world | Nord = row where Race says Nord. Select is a column op. |

Here is a tiny world. Ten fighters. Laid flat, it looks like this. The leftmost column is just the row index — 0-based, the way the column store counts — so ten fighters occupy rows 0 through 9, not 1 through 10.

| row | Race | IsHostile | TwoHanded | … | Archery | Gold |
|---:|---|:---:|---:|---|---:|---:|
| 0 | Nord | 1 | 80 |  | 40 | 100 |
| 1 | Breton | 1 | 55 |  | 70 | 200 |
| 2 | Khajiit | 0 | 70 |  | 85 | 300 |
| 3 | Nord | 0 | 61 |  | 30 | 400 |
| 4 | Imperial | 0 | 90 |  | 50 | 500 |
| 5 | Redguard | 1 | 60 |  | 75 | 600 |
| 6 | Argonian | 0 | 45 |  | 60 | 150 |
| 7 | Nord | 1 | 72 |  | 55 | 350 |
| 8 | Breton | 0 | 40 |  | 65 | 90 |
| 9 | Khajiit | 1 | 88 |  | 90 | 520 |

`Race` is a symbol column: which people live in the row. `IsHostile` is a boolean column — ones and zeros, named the C# way, a yes/no flag on each fighter. `TwoHanded` and `Archery` are numeric skill columns over the same row index; the `…` in between stands for the rest of the skill tree — Block, OneHanded, Sneak, whatever the game registers — same shape, more columns. `Gold` is another numeric column beside them. That is the whole move. An ECS archetype is a table in the SQL sense, or a keyed table in the kdb+ sense — one name for the row set, one vector per attribute, aligned by position. Once the world is a table, "the Nords" is just the rows where `Race` is Nord, and "the hostiles" is just the rows where `IsHostile` is 1 — column operations, not a loop over objects.


### Saying who

In Ano you do not have to memorize FormIDs, object names, or which handle is which. You describe the things by how they are — hostile, Nord, rich, standing in Whiterun — and the host figures out which rows that is. The description *is* the reference.

So let's ask a question. Who is hostile? The boolean column is sitting right there. Type its name:

```haskell
IsHostile
```

And… oh. That can't be right.

```text
1 1 0 0 0 1 0 1 0 1
```

No names. No gold purses. No angry Redguard staring back at you. Just a strip of ones and zeros.

But of course — that *is* the answer. In Ano, a bare selection is a mask: one bit per row, on or off. A mask is like a filter of booleans that activates or deactivates rows of the table. `IsHostile` means "these rows," and these rows are exactly where the column is 1. You asked who, and the language handed you the bitmask.

If you want to see their contents — the actual people under those ones — you use a registered function, `show()`. The comma is the hinge: left side who, right side what to do. For now the "what" is just display; nothing in the world changes yet.

```haskell
IsHostile , show()
```

```text
row  Race       IsHostile  TwoHanded  …  Archery  Gold
0    Nord       1          80            40       100
1    Breton     1          55            70       200
5    Redguard   1          60            75       600
7    Nord       1          72            55       350
9    Khajiit    1          88            90       520
```

There they are. Five hostiles, every column, in table order. `show()` with empty parentheses means the whole width of the row; later you can pass column names to pick and reorder what appears. The selection stayed the same. Only the hinge and the verb turned "which rows?" into "let me see them."

Wow — that's quite a few hostiles. As we can see, they are all a little different. Two Nords and a Breton, a Redguard, a Khajiit; gold from a thin 100 to a fat 600; Archery all over the map from 40 up to 90; some look like bruisers, one looks like they brought a bow to a sword fight. Same flag, very different people.

But what if we only care about the ones skilled enough with the two-handed weapon skill to actually be a threat?

```haskell
IsHostile & TwoHanded > 60
```

```text
1 0 0 0 0 0 0 1 0 1
```

Ah, our ones and zeros yet again. The selection got quite a bit tighter this time. Now we only have three people that match the query. Let's take a closer look:

```haskell
IsHostile & TwoHanded > 60 , show()
```

```text
row  Race       IsHostile  TwoHanded  …  Archery  Gold
0    Nord       1          80            40       100
7    Nord       1          72            55       350
9    Khajiit    1          88            90       520
```

The hostiles who can actually swing. Everyone else is still in the world; they just are not in this description.

But one of these three is not like the others, it seems. That's right… they're poor! Not much use having such a high two-handed skill if they can't even afford to buy a sword. Let's focus on just the two who are a real danger to our dragonborn.

```haskell
IsHostile & TwoHanded > 60 & Gold >= 300 , show()
```

```text
row  Race       IsHostile  TwoHanded  …  Archery  Gold
7    Nord       1          72            55       350
9    Khajiit    1          88            90       520
```

Gold under 300 drops the Nord at row 0 out of the description. Two rows remain: a Nord and a Khajiit, still hostile, still sharp with a two-hander, and solvent enough to matter. Everyone else is still in the world; this sentence simply is not about them.

And finally, we can narrow it down to the character we are actually interested in. The Khajiit is just a caravaneer we pissed off when we stole his goods thirty minutes ago. Now he is halfway across the map. Who we are really looking for is the bandit leader — a Nord warrior in charge of this camp. Stack the description until only he remains:

```haskell
IsHostile & TwoHanded > 60 & Gold >= 300 & Race = :Nord , show()
```

```text
row  Race       IsHostile  TwoHanded  …  Archery  Gold
7    Nord       1          72            55       350
```

One row. The leader.

That `:Nord` is an enum value — one named kind from a small fixed set (Nord, Breton, Khajiit, …). The colon marks the value; bare `Nord` would be something else. `Race = :Nord` is just another mask: rows whose race cell is that kind.

```haskell
Race = :Nord , show(Race, TwoHanded, Gold)
```

```text
row  Race   TwoHanded  Gold
0    Nord   80         100
3    Nord   61         400
7    Nord   72         350
```

Three Nords in the camp. The leader was the one who also cleared the other gates — hostile, skilled, solvent.


### `&` vs `@`

For `Merchant @ Whiterun` versus `Merchant & Whiterun`: they aren't different.

`@` is the locative — where the selection stands, not what it is (日本語 `で`). Demo 004:

```haskell
Merchant @ Whiterun , Gold += 5000
```

In Steel's mask emitter, plain `mask @ mask` and `mask & mask` both become the same BQN.

`&` / `|`:

```rust
NodeKind::And(a, b) | NodeKind::Or(a, b) => {
    let am = self.emit_mask(a)?;
    let bm = self.emit_mask(b)?;
    Ok(format!("({}{}{})", am, if matches!(nd.kind, NodeKind::And(..)) { "∧" } else { "∨" }, bm))
}
```

`@` with no frame origin:

```rust
NodeKind::Scope { l, r, origin: None } => {
    if matches!(r.kind, NodeKind::Shape(_)) {
        // frame scope: predicate over the lattice
        return self.emit_mask(l);
    }
    let a = self.emit_mask(l)?;
    let b = self.emit_mask(r)?;
    Ok(format!("({}∧{})", a, b))
}
```

So:

| Surface | AST | Emitted mask |
|---|---|---|
| `Merchant & Whiterun` | `And` | `(merchant∧whiterun)` |
| `Merchant @ Whiterun` | `Scope` | `(merchant∧whiterun)` |

Same bits. Same gather. Same effect. The locative story is rhetoric over identical lowering.

Where they actually diverge is not at `Merchant @ Whiterun`. It is at the other `@` shapes:

| Form | What happens | `&` equivalent? |
|---|---|---|
| `Merchant @ Whiterun` | mask ∧ mask | yes, identical |
| `+/ Gold @ Nord` | fold splits `Scope`: gather `Gold` under mask `Nord` → one scalar | no — `+/ Gold & Nord` is a different parse/job |
| `pred @ 8 8` | right is a shape; emit just left over the lattice | no |
| `Oil @ blast(5) at p` | runs frame fn, then ∧ with left | no |
| `Adj @ row` | special per-row fiber fold | no |

Fold path is the real fork — `@` is peeled apart as operand/scope:

```rust
// split operand @ scope
let (x, scope): (&Node, Option<&Node>) =
    if let NodeKind::Scope { l, r, .. } = &operand.kind { (l, Some(r)) } else { (operand, None) };
// ...
Some(sc) => {
    // mask scope
    let msk = self.emit_mask(sc)?;
    let xv = self.emit_val(x, Mode::World)?;
    // ...
    gathered = format!("({}/{})", m2, xv.v);
```

Also precedence: `@` is tighter than `&`, so `Cheese @ cellar & Aged > 3mo` parses as `(Cheese @ cellar) & (Aged > 3mo)`.

Bottom line: `Merchant @ Whiterun` vs `Merchant & Whiterun` is one spelling difference, zero semantic difference in the current compiler. `@` only becomes a different machine when it scopes a fold, a lattice shape, or an anchored frame.
