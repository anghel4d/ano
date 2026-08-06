# Tour

Welcome to AnoLang. This is a walk through the language as it actually is — not the mythology around it — one piece at a time, starting from the world it talks about before any syntax shows up.

People usually think of a video game as a pile of objects. A Nord is an instance of some class or actor type; he has fields for health and gold, methods to take damage or trade, and a place in a scene graph or entity list. To change every Nord who meets some condition, you walk that list, ask each object what it is, and mutate the ones that pass. The mental model is object orientation: identity first, data hanging off it, behavior as methods on the thing.

There is another view, and it falls out almost naturally from what game engines started calling ECS. Instead of a Nord-the-object, the world is a columnar store: rows are entities, columns are components, and an archetype is just the set of columns that travel together. A Nord is not a class instance. He is a row that happens to be present in the Nord column — and in Gold, and in TwoHanded — the same way a row in a SQL table is present under every column of that table, or a row in a kdb+ table sits under each of its vectors.

Here is a tiny world. Six fighters. Three columns. The Nord archetype, laid flat, looks like this:

| row | Nord | TwoHanded | Gold |
|---:|:---:|---:|---:|
| 0 | 1 | 80 | 100 |
| 1 | 1 | 55 | 200 |
| 2 | 0 | 70 | 300 |
| 3 | 1 | 61 | 400 |
| 4 | 0 | 90 | 500 |
| 5 | 1 | 60 | 600 |

`Nord` is a boolean column: who is one. `TwoHanded` and `Gold` are numeric columns over the same row index. That is the whole move. An ECS archetype is a table in the SQL sense, or a keyed table in the kdb+ sense — one name for the row set, one vector per attribute, aligned by position. Once the world is a table, selecting Nords is a column operation, not a loop over objects. The language comes after that fact.


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
