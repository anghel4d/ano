# Tour

A walk through Ano as it actually is. One section at a time.


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
