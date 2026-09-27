// Temporary values have lexical scope and no implicit entity-row lineage.
use super::*;

impl Em<'_> {
    pub(super) fn emit_construct(
        &mut self,
        node: &Node,
        bindings: &mut Vec<(Symbol, String)>,
        depth: usize,
    ) -> R<String> {
        if depth >= 64 {
            return Err(fail(node.line, "construction nested too deeply"));
        }
        let next = depth + 1;
        match &node.kind {
            NodeKind::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|item| self.emit_construct(item, bindings, next))
                    .collect::<R<Vec<_>>>()?;
                Ok(format!("⟨{}⟩", items.join(", ")))
            }
            NodeKind::Range { start, end } => {
                let a = self.emit_construct(start, bindings, next)?;
                let b = self.emit_construct(end, bindings, next)?;
                // Finite integral bounds; descending unit-step ranges are empty.
                Ok(format!(
                    r#"({{a‿b←𝕩 ⋄ "range bounds must be finite integers" ! (0=≠≢a)∧(0=≠≢b)∧(a=⌊a)∧(b=⌊b)∧(∞>|a)∧∞>|b ⋄ a+↕0⌈1+b-a}} ⟨{a}, {b}⟩)"#
                ))
            }
            NodeKind::Generate { source, name, body } => {
                let source = self.emit_construct(source, bindings, next)?;
                let local = format!("anoLocal{}", bindings.len());
                bindings.push((*name, local.clone()));
                let result = self.emit_construct(body, bindings, next);
                bindings.pop();
                let result = result?;
                Ok(format!(
                    r#"({{{local}←𝕩 ⋄ {result}}}¨{{"generator source must be a tuple or column" ! 1=≠≢𝕩 ⋄ 𝕩}} {source})"#
                ))
            }
            NodeKind::Name(name) => {
                if let Some((_, value)) = bindings.iter().rev().find(|(bound, _)| bound == name) {
                    return Ok(value.clone());
                }
                if let Some(def) = self.find_def(*name) {
                    // Definitions do not capture the caller's comprehension bindings.
                    return self.emit_construct(def_body(def), &mut Vec::new(), next);
                }
                let value = self.emit_val(node, Mode::World)?;
                if let Some(guard) = value.g {
                    if value.unit {
                        return Err(fail(node.line, "absent scalar in temporary construction"));
                    }
                    return Ok(format!("({guard}/{})", value.v));
                }
                Ok(value.v)
            }
            NodeKind::Num(_) | NodeKind::Counter { .. } | NodeKind::Sym(_) | NodeKind::Str(_) => {
                Ok(self.emit_val(node, Mode::World)?.v)
            }
            NodeKind::Arith { op, l, r } => {
                let a = self.emit_construct(l, bindings, next)?;
                let b = self.emit_construct(r, bindings, next)?;
                let operation = match op {
                    ArithOp::Add => "a+b",
                    ArithOp::Sub => "a-b",
                    ArithOp::Mul => "a×b",
                    ArithOp::Div => "a÷b",
                    ArithOp::Mod => "b|a",
                };
                Ok(format!(
                    r#"({{a‿b←𝕩 ⋄ "construction arithmetic requires numeric scalars" ! (0=≠≢a)∧(0=≠≢b)∧(1=•Type a)∧1=•Type b ⋄ v←{operation} ⋄ "NaN in construction" ! v=v ⋄ v}} ⟨{a}, {b}⟩)"#
                ))
            }
            NodeKind::Cmp { op, l, r } => {
                let a = self.emit_construct(l, bindings, next)?;
                let b = self.emit_construct(r, bindings, next)?;
                let op = match op {
                    CmpOp::Eq => "≡",
                    CmpOp::Ne => "≢",
                    CmpOp::Lt => "<",
                    CmpOp::Gt => ">",
                    CmpOp::Le => "≤",
                    CmpOp::Ge => "≥",
                };
                Ok(format!("({a}{op}{b})"))
            }
            NodeKind::Call { callee, args } => {
                let spelling = self.rs(*callee).to_string();
                if spelling == "lazy" && self.find(&spelling).is_none() {
                    return Err(fail(
                        node.line,
                        "lazy() is reserved for deferred construction; not implemented yet",
                    ));
                }
                if self.defs.iter().any(
                    |def| matches!(def.kind, NodeKind::DefStmt { name, .. } if name == *callee),
                ) || bindings.iter().any(|(name, _)| name == callee)
                {
                    return Err(fail(node.line, "a value binding is not callable"));
                }
                let mut signature = None;
                let function = if let Some(index) = self.find(&spelling) {
                    match &self.ent(index).kind {
                        RegEntryKind::Fn { body: Some(_) }
                            if !crate::show::registered(self.ent(index)) =>
                        {
                            self.fnv(index)
                        }
                        RegEntryKind::TypedFn { descriptor, .. } => {
                            if descriptor.effects != crate::EffectSet::default()
                                || descriptor.determinism != crate::Determinism::Deterministic
                                || descriptor.signature.output == crate::RegType::Unit
                            {
                                return Err(fail(node.line, "temporary construction requires a pure deterministic value callable"));
                            }
                            if descriptor.signature.inputs.len() != args.len() {
                                return Err(fail(node.line, "construction callable argument count does not match its signature"));
                            }
                            signature = Some(descriptor.signature.clone());
                            self.fnv(index)
                        }
                        _ => {
                            return Err(fail(
                                node.line,
                                "construction requires a pure value callable with a backend body",
                            ))
                        }
                    }
                } else {
                    match spelling.as_str() {
                        "abs" => "|".into(),
                        "sin" => "•math.Sin".into(),
                        _ => {
                            return Err(fail(
                                node.line,
                                format!("unregistered callable '{spelling}'"),
                            ))
                        }
                    }
                };
                let mut args = args
                    .iter()
                    .map(|arg| self.emit_construct(arg, bindings, next))
                    .collect::<R<Vec<_>>>()?;
                if let Some(signature) = &signature {
                    for (arg, carrier) in args.iter_mut().zip(&signature.inputs) {
                        *arg = checked_local(arg, carrier, node.line)?;
                    }
                }
                let result = match args.as_slice() {
                    [a] => format!("({function} {a})"),
                    [a, b] if self.find(&spelling).is_some() => format!("({a} {function} {b})"),
                    _ => {
                        return Err(fail(
                            node.line,
                            "construction callable requires its unary or binary argument list",
                        ))
                    }
                };
                match signature {
                    Some(signature) => checked_local(&result, &signature.output, node.line),
                    None => Ok(result),
                }
            }
            _ => Err(fail(
                node.line,
                "unsupported expression in temporary construction",
            )),
        }
    }
}

// Registered scalar signatures remain exact at the local invocation boundary.
fn checked_local(value: &str, carrier: &crate::RegType, line: i32) -> R<String> {
    use crate::RegType::*;
    let condition = match carrier {
        Num => "(1=•Type 𝕩)∧𝕩=𝕩",
        Int => "(1=•Type 𝕩)∧(𝕩=⌊𝕩)∧9007199254740992≥|𝕩",
        Nat => "(1=•Type 𝕩)∧(𝕩=⌊𝕩)∧(𝕩≥0)∧𝕩≤9007199254740992",
        Mask => "(1=•Type 𝕩)∧(𝕩=0)∨𝕩=1",
        Char => "2=•Type 𝕩",
        _ => {
            return Err(fail(
                line,
                "this callable carrier has no local construction ABI",
            ))
        }
    };
    Ok(format!(
        r#"({{"local callable carrier mismatch" ! (0=≠≢𝕩)∧({condition}) ⋄ 𝕩}} {value})"#
    ))
}
