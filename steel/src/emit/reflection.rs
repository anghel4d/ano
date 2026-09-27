// Reflection values are shared opaque namespaces within a compiled schema snapshot.
// They expose declarations, never the columns' data or executable callable bodies.
use super::*;

fn text(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn symbol(value: &str) -> String {
    format!("(<{})", text(value))
}
fn column_type(ty: ColType) -> &'static str {
    match ty {
        ColType::Num => "num",
        ColType::Bool => "bool",
        ColType::Nat => "nat",
        ColType::Int => "int",
        ColType::Sym => "sym",
        ColType::Char => "char",
    }
}

impl Em<'_> {
    pub(super) fn is_reflection(&self, node: &Node, depth: usize) -> bool {
        if depth >= 64 {
            return false;
        }
        match &node.kind {
            NodeKind::Reference { .. } => true,
            NodeKind::Name(name) => self
                .find_def(*name)
                .is_some_and(|def| self.is_reflection(def_body(def), depth + 1)),
            NodeKind::Hop { l, .. } | NodeKind::Not(l) => self.is_reflection(l, depth + 1),
            NodeKind::Arith { l, r, .. }
            | NodeKind::Cmp { l, r, .. }
            | NodeKind::And(l, r)
            | NodeKind::Or(l, r) => {
                self.is_reflection(l, depth + 1) || self.is_reflection(r, depth + 1)
            }
            _ => false,
        }
    }

    pub(super) fn reference_category(&self, node: &Node, depth: usize) -> Option<RefCategory> {
        if depth >= 64 {
            return None;
        }
        match &node.kind {
            NodeKind::Reference { category, .. } => Some(*category),
            NodeKind::Name(name) => self
                .find_def(*name)
                .and_then(|def| self.reference_category(def_body(def), depth + 1)),
            _ => None,
        }
    }

    pub(super) fn reflection_property(&self, node: &Node, depth: usize) -> Option<String> {
        if depth >= 64 {
            return None;
        }
        match &node.kind {
            NodeKind::Hop { l, r } if self.is_reflection(l, depth + 1) => match r.kind {
                NodeKind::Name(property) => Some(self.rs(property).to_string()),
                _ => None,
            },
            NodeKind::Name(name) => self
                .find_def(*name)
                .and_then(|def| self.reflection_property(def_body(def), depth + 1)),
            _ => None,
        }
    }

    pub(super) fn check_reference_property(
        &self,
        category: RefCategory,
        name: &str,
        property: &str,
        line: i32,
    ) -> R<()> {
        let allowed = match category {
            RefCategory::Column => matches!(
                property,
                "name" | "type" | "domain" | "range" | "unique" | "constraints"
            ),
            RefCategory::Function => matches!(
                property,
                "name"
                    | "inputs"
                    | "result"
                    | "reads"
                    | "writes"
                    | "arity"
                    | "effects"
                    | "determinism"
                    | "trust"
                    | "associative"
                    | "commutative"
                    | "monotonic"
            ),
        };
        if !allowed {
            return Err(fail(
                line,
                format!("property '{property}' is unavailable for this declaration category"),
            ));
        }
        if category == RefCategory::Function
            && !matches!(
                property,
                "name" | "associative" | "commutative" | "monotonic"
            )
            && !self
                .find(name)
                .is_some_and(|i| matches!(self.ent(i).kind, RegEntryKind::TypedFn { .. }))
        {
            return Err(fail(line, format!("'{name}' has no declared signature/footprint metadata; register a typed callable")));
        }
        Ok(())
    }

    pub(super) fn emit_reference(
        &mut self,
        category: RefCategory,
        name: &str,
        line: i32,
    ) -> R<String> {
        let canonical =
            crate::reference::resolve(self.reg, name, category).map_err(|e| fail(line, e.msg))?;
        if let Some((_, _, value)) = self
            .reference_values
            .iter()
            .find(|(kind, name, _)| *kind == category && name == &canonical)
        {
            return Ok(value.clone());
        }
        let index = self.find(&canonical);
        let mut fields = vec![format!("name⇐{}", text(&canonical))];
        if category == RefCategory::Function {
            let descriptor = if index.is_none() {
                reducer::canonical(&canonical, reducer::Form::Fold)
            } else {
                None
            };
            let laws = descriptor.as_ref().and_then(|d| d.reducer());
            let word = |law| match law {
                reducer::LawStatus::Holds => "holds",
                reducer::LawStatus::DoesNotHold => "fails",
                reducer::LawStatus::Undeclared => "undeclared",
            };
            fields.push(format!(
                "associative⇐{}",
                symbol(word(
                    laws.map_or(reducer::LawStatus::Undeclared, |d| d.associativity)
                ))
            ));
            fields.push(format!(
                "commutative⇐{}",
                symbol(word(
                    laws.map_or(reducer::LawStatus::Undeclared, |d| d.commutativity)
                ))
            ));
            fields.push(format!("monotonic⇐{}", symbol("undeclared")));
        }
        if let Some(index) = index {
            let entry = self.ent(index).clone();
            match (&entry.kind, category) {
                (RegEntryKind::TypedFn { descriptor, .. }, RefCategory::Function) => {
                    let inputs = descriptor
                        .signature
                        .inputs
                        .iter()
                        .map(|ty| symbol(crate::registry::reg_type_word(ty)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    fields.push(format!("inputs⇐⟨{inputs}⟩"));
                    fields.push(format!("arity⇐{}", descriptor.signature.inputs.len()));
                    let mut effects = Vec::new();
                    if descriptor.effects.read {
                        effects.push(symbol("read"));
                    }
                    if descriptor.effects.write {
                        effects.push(symbol("write"));
                    }
                    if descriptor.effects.service {
                        effects.push(symbol("service"));
                    }
                    fields.push(format!("effects⇐⟨{}⟩", effects.join(", ")));
                    fields.push(format!(
                        "determinism⇐{}",
                        symbol(match descriptor.determinism {
                            crate::Determinism::Deterministic => "deterministic",
                            crate::Determinism::Snapshot => "snapshot",
                            crate::Determinism::Nondeterministic => "nondeterministic",
                        })
                    ));
                    fields.push(format!(
                        "trust⇐{}",
                        symbol(match descriptor.trust {
                            crate::TrustBoundary::Trusted => "trusted",
                            crate::TrustBoundary::Checked => "checked",
                        })
                    ));
                    fields.push(format!(
                        "result⇐{}",
                        symbol(crate::registry::reg_type_word(&descriptor.signature.output))
                    ));
                    for (label, columns) in
                        [("reads", &descriptor.reads), ("writes", &descriptor.writes)]
                    {
                        let refs = columns
                            .iter()
                            .map(|name| self.emit_reference(RefCategory::Column, name, line))
                            .collect::<R<Vec<_>>>()?;
                        fields.push(format!("{label}⇐⟨{}⟩", refs.join(", ")));
                    }
                }
                (kind, RefCategory::Column) => {
                    let (carrier, domain) = match kind {
                        RegEntryKind::Col { ty, .. } => (column_type(*ty).to_string(), "entity"),
                        RegEntryKind::Field { ty, .. } => (column_type(*ty).to_string(), "lattice"),
                        RegEntryKind::Rel { .. } => ("entity".into(), "entity"),
                        RegEntryKind::SRel { .. } => ("entities".into(), "entity"),
                        RegEntryKind::Tag { .. } | RegEntryKind::AliasMask { .. } => {
                            ("bool".into(), "entity")
                        }
                        RegEntryKind::Array { descriptor } => (
                            crate::registry::reg_type_word(&descriptor.carrier).to_string(),
                            match descriptor.domain {
                                crate::ArrayDomain::Scalar => "scalar",
                                crate::ArrayDomain::Entity => "entity",
                                crate::ArrayDomain::Fixed(_) => "fixed",
                            },
                        ),
                        _ => return Err(fail(line, "not a column declaration")),
                    };
                    fields.push(format!("type⇐{}", symbol(&carrier)));
                    fields.push(format!("domain⇐{}", symbol(domain)));
                    let mut range = match kind {
                        RegEntryKind::Col { rng, .. } => *rng,
                        _ => None,
                    };
                    if let RegEntryKind::Array { descriptor } = kind {
                        if let crate::RegType::Named(name) = &descriptor.carrier {
                            if let Some(i) = self.find(name) {
                                if let RegEntryKind::Ctor { descriptor } = &self.ent(i).kind {
                                    if let crate::ConstructorRefinement::Range { lo, hi } =
                                        descriptor.refinement
                                    {
                                        range = Some((lo, hi));
                                    }
                                }
                            }
                        }
                    }
                    fields.push(format!(
                        "range⇐{}",
                        range.map_or_else(
                            || "⟨⟩".into(),
                            |(lo, hi)| format!("⟨{}, {}⟩", num_lit(lo), num_lit(hi))
                        )
                    ));
                    let unique = matches!(kind, RegEntryKind::Col { uniq: true, .. });
                    fields.push(format!("unique⇐{}", u8::from(unique)));
                    let mut constraints = Vec::new();
                    if matches!(carrier.as_str(), "bool" | "mask" | "nat" | "int") {
                        constraints.push(symbol("integer"));
                    }
                    if matches!(carrier.as_str(), "bool" | "mask" | "nat" | "int")
                        || range.is_some_and(|(lo, hi)| lo.is_finite() && hi.is_finite())
                    {
                        constraints.push(symbol("finite"));
                    }
                    if matches!(carrier.as_str(), "bool" | "mask" | "nat")
                        || range.is_some_and(|(lo, _)| lo >= 0.0)
                    {
                        constraints.push(symbol("nonnegative"));
                    }
                    if unique {
                        constraints.push(symbol("unique"));
                    }
                    fields.push(format!("constraints⇐⟨{}⟩", constraints.join(", ")));
                }
                _ => {}
            }
        }
        let mut suffix = self.reference_values.len();
        let value = loop {
            let candidate = format!("anoReflection{suffix}");
            if self.find(&candidate).is_none()
                && !self
                    .reference_values
                    .iter()
                    .any(|(_, _, value)| value == &candidate)
            {
                break candidate;
            }
            suffix += 1;
        };
        self.need_declaration(&format!("{value} ← {{{}}}", fields.join(" ⋄ ")));
        self.reference_values
            .push((category, canonical, value.clone()));
        Ok(value)
    }
}
