// Temporary values have lexical scope and no implicit entity-row lineage.
use super::*;

impl Em<'_> {
    pub(super) fn emit_construct(
        &mut self,
        node: &Node,
        bindings: &mut Vec<(Symbol, String, bool)>,
        depth: usize,
    ) -> R<String> {
        if depth >= 64 {
            return Err(fail(node.line, "construction nested too deeply"));
        }
        let next = depth + 1;
        match &node.kind {
            NodeKind::Reference { category, name } => self.emit_reference(*category, self.rs(*name).to_string().as_str(), node.line),
            NodeKind::Hop { l, r } => {
                let NodeKind::Name(property) = r.kind else { return Err(fail(node.line, "reflection property must be a name")); };
                let property = self.rs(property).to_string();
                if !matches!(property.as_str(), "name" | "type" | "domain" | "inputs" | "result" | "reads" | "writes" | "arity" | "effects" | "determinism" | "trust" | "associative" | "commutative" | "monotonic" | "range" | "unique" | "constraints") {
                    return Err(fail(node.line, format!("unknown reflection property '{property}'")));
                }
                if let NodeKind::Reference { category, name } = l.kind {
                    self.check_reference_property(category, self.rs(name), &property, node.line)?;
                }
                let value = self.emit_construct(l, bindings, next)?;
                Ok(format!("({value}).{property}"))
            }
            NodeKind::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|item| self.emit_construct(item, bindings, next))
                    .collect::<R<Vec<_>>>()?;
                Ok(format!("⟨{}⟩", items.join(", ")))
            }
            NodeKind::Range { start, end } => {
                if self.entity_value(start, bindings, next)?
                    || self.entity_value(end, bindings, next)?
                {
                    return Err(fail(
                        node.line,
                        "entity references are not numeric range bounds",
                    ));
                }
                let a = self.emit_construct(start, bindings, next)?;
                let b = self.emit_construct(end, bindings, next)?;
                // Finite integral bounds; descending unit-step ranges are empty.
                Ok(format!(
                    r#"({{a‿b←𝕩 ⋄ "range bounds must be finite integers" ! (0=≠≢a)∧(0=≠≢b)∧(a=⌊a)∧(b=⌊b)∧(∞>|a)∧∞>|b ⋄ a+↕0⌈1+b-a}} ⟨{a}, {b}⟩)"#
                ))
            }
            NodeKind::Generate { clauses, body } => {
                self.emit_qualifiers(clauses, body, bindings, next)
            }
            NodeKind::Name(name) => {
                if let Some((_, value, _)) =
                    bindings.iter().rev().find(|(bound, _, _)| bound == name)
                {
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
                if self.entity_value(l, bindings, next)? || self.entity_value(r, bindings, next)? {
                    return Err(fail(node.line, "entity references do not admit arithmetic"));
                }
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
                let left_entity = self.entity_value(l, bindings, next)?;
                let right_entity = self.entity_value(r, bindings, next)?;
                if (left_entity || right_entity)
                    && (!left_entity || !right_entity || !matches!(op, CmpOp::Eq | CmpOp::Ne))
                {
                    return Err(fail(
                        node.line,
                        "entity references admit equality only with other entity references",
                    ));
                }
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
            NodeKind::Not(inner) => {
                if self.entity_value(inner, bindings, next)? {
                    return Err(fail(node.line, "entity references are not Boolean guards"));
                }
                let value = self.emit_construct(inner, bindings, next)?;
                Ok(format!(
                    "(¬{})",
                    checked_local(&value, &crate::RegType::Mask, node.line)?
                ))
            }
            NodeKind::And(left, right) | NodeKind::Or(left, right) => {
                let a = self.emit_construct(left, bindings, next)?;
                let b = self.emit_construct(right, bindings, next)?;
                if self.entity_value(left, bindings, next)?
                    || self.entity_value(right, bindings, next)?
                {
                    return Err(fail(
                        node.line,
                        "entity references do not admit numeric operations",
                    ));
                }
                let a = checked_local(&a, &crate::RegType::Num, node.line)?;
                let b = checked_local(&b, &crate::RegType::Num, node.line)?;
                Ok(format!(
                    "({a}{}{b})",
                    if matches!(node.kind, NodeKind::And(..)) {
                        "⌊"
                    } else {
                        "⌈"
                    }
                ))
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
                ) || bindings.iter().any(|(name, _, _)| name == callee)
                {
                    return Err(fail(node.line, "a value binding is not callable"));
                }
                if self.find(&spelling).is_none()
                    && matches!(spelling.as_str(), "entities" | "selection")
                {
                    if args.len() != 1 {
                        return Err(fail(node.line, format!("{spelling} expects 1 argument")));
                    }
                    return if spelling == "entities" {
                        fn uses_local(node: &Node, bindings: &[(Symbol, String, bool)]) -> bool {
                            matches!(&node.kind, NodeKind::Name(name) if bindings.iter().any(|(bound, _, _)| bound == name))
                                || children(node)
                                    .iter()
                                    .any(|child| uses_local(child, bindings))
                        }
                        if uses_local(&args[0], bindings) {
                            return Err(fail(node.line, "entities() needs a world selection independent of local bindings; use a dependent tuple source instead"));
                        }
                        self.emit_entities(&args[0])
                    } else {
                        if !self.entity_sequence(&args[0], bindings, next)? {
                            return Err(fail(
                                node.line,
                                "selection() requires a flat collection of entity references",
                            ));
                        }
                        let refs = self.emit_construct(&args[0], bindings, next)?;
                        Ok(format!("({}∊{refs})", self.id_col()))
                    };
                }
                let mut signature = None;
                let function = if let Some(index) = self.find(&spelling) {
                    match &self.ent(index).kind {
                        RegEntryKind::Fn { body: Some(body) }
                            if body.trim_start().starts_with('{')
                                && !crate::show::registered(self.ent(index)) =>
                        {
                            self.fnv(index)
                        }
                        RegEntryKind::TypedFn { descriptor, .. } => {
                            if descriptor.effects.write
                                || descriptor.effects.service
                                || descriptor.determinism == crate::Determinism::Nondeterministic
                                || descriptor.signature.output == crate::RegType::Unit
                            {
                                return Err(fail(node.line, "temporary construction requires a deterministic or snapshot read-only value callable"));
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
                if let Some(signature) = &signature {
                    for (arg, carrier) in args.iter().zip(&signature.inputs) {
                        if self.entity_value(arg, bindings, next)?
                            != (*carrier == crate::RegType::Entity)
                        {
                            return Err(fail(
                                node.line,
                                "entity callable arguments must retain entity provenance",
                            ));
                        }
                    }
                    if signature.output == crate::RegType::Entity {
                        return Err(fail(
                            node.line,
                            "callable entity results require a checked host reference boundary",
                        ));
                    }
                }
                let mut args = args
                    .iter()
                    .map(|arg| self.emit_construct(arg, bindings, next))
                    .collect::<R<Vec<_>>>()?;
                if let Some(signature) = &signature {
                    for (arg, carrier) in args.iter_mut().zip(&signature.inputs) {
                        if *carrier != crate::RegType::Entity {
                            *arg = checked_local(arg, carrier, node.line)?;
                        }
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
    // Each recursive clause returns a result list. Only qualifier expansion flattens;
    // the yield is enclosed once, preserving every nested tuple supplied by the body.
    fn emit_qualifiers(
        &mut self,
        clauses: &[Node],
        body: &Node,
        bindings: &mut Vec<(Symbol, String, bool)>,
        depth: usize,
    ) -> R<String> {
        if depth >= 64 {
            return Err(fail(body.line, "construction nested too deeply"));
        }
        let Some((clause, rest)) = clauses.split_first() else {
            return Ok(format!(
                "⟨{}⟩",
                self.emit_construct(body, bindings, depth + 1)?
            ));
        };
        if let NodeKind::Binder { name, source } = &clause.kind {
            let entity = !matches!(&source.kind, NodeKind::Tuple(items) if items.is_empty())
                && self.entity_sequence(source, bindings, depth + 1)?;
            let source = self.emit_construct(source, bindings, depth + 1)?;
            let local = format!("anoLocal{}", bindings.len());
            bindings.push((*name, local.clone(), entity));
            let result = self.emit_qualifiers(rest, body, bindings, depth + 1);
            bindings.pop();
            let result = result?;
            Ok(format!(
                r#"(∾{{{local}←𝕩 ⋄ {result}}}¨{{"generator source must be a tuple or column" ! 1=≠≢𝕩 ⋄ 𝕩}} {source})"#
            ))
        } else {
            if self.entity_value(clause, bindings, depth + 1)? {
                return Err(fail(
                    clause.line,
                    "entity references are not Boolean guards",
                ));
            }
            let guard = self.emit_construct(clause, bindings, depth + 1)?;
            let guard = checked_local(&guard, &crate::RegType::Mask, clause.line)?;
            let result = self.emit_qualifiers(rest, body, bindings, depth + 1)?;
            Ok(format!("({{𝕩 ? {result} ; ⟨⟩}} {guard})"))
        }
    }

    fn entity_value(
        &self,
        node: &Node,
        bindings: &[(Symbol, String, bool)],
        depth: usize,
    ) -> R<bool> {
        if depth >= 64 {
            return Err(fail(node.line, "construction nested too deeply"));
        }
        Ok(match &node.kind {
            NodeKind::Name(name) => bindings
                .iter()
                .rev()
                .find(|(bound, _, _)| bound == name)
                .is_some_and(|(_, _, entity)| *entity),
            _ => false,
        })
    }

    fn entity_sequence(
        &self,
        node: &Node,
        bindings: &[(Symbol, String, bool)],
        depth: usize,
    ) -> R<bool> {
        if depth >= 64 {
            return Err(fail(node.line, "construction nested too deeply"));
        }
        match &node.kind {
            NodeKind::Call { callee, args }
                if self.rs(*callee) == "entities"
                    && self.find("entities").is_none()
                    && self.find_def(*callee).is_none() =>
            {
                Ok(args.len() == 1)
            }
            NodeKind::Name(name) => {
                if bindings.iter().any(|(bound, _, _)| bound == name) {
                    return Ok(false);
                }
                match self.find_def(*name) {
                    Some(def) => self.entity_sequence(def_body(def), &[], depth + 1),
                    None => Ok(false),
                }
            }
            NodeKind::Tuple(items) => {
                for item in items {
                    if !self.entity_value(item, bindings, depth + 1)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            NodeKind::Generate { clauses, body } => {
                let mut locals = bindings.to_vec();
                for clause in clauses {
                    if let NodeKind::Binder { name, source } = &clause.kind {
                        let entity = self.entity_sequence(source, &locals, depth + 1)?;
                        locals.push((*name, String::new(), entity));
                    }
                }
                self.entity_value(body, &locals, depth + 1)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn emit_entities(&mut self, selection: &Node) -> R<String> {
        if self.fr.kind != FrameKind::Ent {
            return Err(fail(selection.line, "entities() requires the entity world"));
        }
        let mask = self.emit_mask(selection)?;
        let ids = self.id_col();
        // Validate the identity map even when a registry assigns roles without unique metadata.
        Ok(format!(
            r#"({{m←𝕩 ⋄ ids←{ids} ⋄ "entity identities must be unique" ! (≠ids)=≠⍷ids ⋄ "entity selection must be a world mask" ! (≢m)≡⟨anoN⟩ ⋄ "entity selection must be boolean" ! ∧´(m=0)∨m=1 ⋄ m/ids}} {mask})"#
        ))
    }

    pub(super) fn emit_selection(&mut self, refs: &Node) -> R<String> {
        if self.fr.kind != FrameKind::Ent {
            return Err(fail(refs.line, "selection() requires the entity world"));
        }
        if !self.entity_sequence(refs, &[], 0)? {
            return Err(fail(
                refs.line,
                "selection() requires a flat collection of entity references",
            ));
        }
        let refs = self.emit_construct(refs, &mut Vec::new(), 0)?;
        Ok(format!("({}∊{refs})", self.id_col()))
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
