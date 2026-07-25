//! One carrier-directed operation table for direct dyads, folds, and scans.
//!
//! Ano reductions are ordered.  Homogeneous folds use the unseeded left recurrence; BQN's
//! right fold is therefore lowered as `F˜´⌽x`.  Count and average are prefix machines rather
//! than pretending to be homogeneous binary operations, so they are a separate descriptor:
//! a reducer with a hole where its step belongs would be a false abstraction.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    Mask,
    Number,
    Presence,
}

/// The surface form a head was spelled in.  One head resolves once, against one form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Direct,
    Fold,
    Scan,
    ScanAlong,
}

impl Form {
    fn name(self) -> &'static str {
        match self {
            Form::Direct => "direct",
            Form::Fold => "fold",
            Form::Scan => "scan",
            Form::ScanAlong => "scan-along",
        }
    }
}

/// Algebraic laws are declared, never inferred.  `Undeclared` is not a denial: it admits exact
/// left execution and refuses every strategy that would regroup or reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawStatus {
    Holds,
    DoesNotHold,
    Undeclared,
}

/// The execution plan a lowering intends.  Steel lowers `ExactLeft` everywhere today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    ExactLeft,
    Regroup,
    Reorder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineKind {
    Count,
    Average,
}

/// An identity belongs to a carrier, never to a surface glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedIdentity {
    pub carrier: Carrier,
    pub bqn: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyFold {
    Identity(TypedIdentity),
    NoRow,
}

impl EmptyFold {
    pub fn identity(&self) -> Option<TypedIdentity> {
        match self {
            EmptyFold::Identity(value) => Some(*value),
            EmptyFold::NoRow => None,
        }
    }
}

/// A homogeneous unseeded reducer: `A × A → A` under the exact left recurrence.  `step` is the
/// BQN dyad; it is empty exactly when a registered function supplies the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReducerDesc {
    pub spelling: String,
    pub input: Carrier,
    pub output: Carrier,
    pub step: &'static str,
    pub identity: Option<TypedIdentity>,
    pub associativity: LawStatus,
    pub commutativity: LawStatus,
}

/// A stateful prefix machine: state `S`, an emit per prefix, a finish for the fold result, and
/// an explicit empty-fold law.  Average's state is the pair `(sum,count)` over its state carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixMachineDesc {
    pub spelling: String,
    pub kind: MachineKind,
    pub input: Carrier,
    pub state: Carrier,
    pub output: Carrier,
    pub empty_fold: EmptyFold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpDesc {
    Reducer(ReducerDesc),
    Machine(PrefixMachineDesc),
}

impl ReducerDesc {
    pub fn direct_step(&self) -> Option<&'static str> {
        if self.step.is_empty() { None } else { Some(self.step) }
    }

    /// The BQN fold spelling.  `None` when the registry supplies the step.
    pub fn fold_glyph(&self) -> Option<String> {
        self.direct_step().map(|step| format!("{}´", step))
    }

    /// The BQN scan spelling.  BQN's scan modifier is natively left-to-right.
    pub fn scan_glyph(&self) -> Option<String> {
        self.direct_step().map(|step| format!("{}`", step))
    }

    pub fn empty_identity(&self) -> Option<&'static str> {
        self.identity.map(|identity| identity.bqn)
    }
}

impl OpDesc {
    pub fn spelling(&self) -> &str {
        match self {
            OpDesc::Reducer(desc) => &desc.spelling,
            OpDesc::Machine(desc) => &desc.spelling,
        }
    }

    pub fn reducer(&self) -> Option<&ReducerDesc> {
        match self {
            OpDesc::Reducer(desc) => Some(desc),
            OpDesc::Machine(..) => None,
        }
    }

    pub fn machine(&self) -> Option<&PrefixMachineDesc> {
        match self {
            OpDesc::Machine(desc) => Some(desc),
            OpDesc::Reducer(..) => None,
        }
    }

    /// The registered empty-fold identity of this operation, per carrier.  No spelling is
    /// consulted, and no infinity is ever manufactured for the numeric extrema.
    pub fn empty_identity(&self) -> Option<&'static str> {
        match self {
            OpDesc::Reducer(desc) => desc.empty_identity(),
            OpDesc::Machine(desc) => desc.empty_fold.identity().map(|identity| identity.bqn),
        }
    }

    pub fn fold_glyph(&self) -> Option<String> {
        self.reducer().and_then(ReducerDesc::fold_glyph)
    }

    pub fn scan_glyph(&self) -> Option<String> {
        self.reducer().and_then(ReducerDesc::scan_glyph)
    }
}

fn is_builtin(spelling: &str) -> bool {
    matches!(spelling, "+" | "-" | "*" | "/" | "|" | "&" | "max" | "min" | "#" | "avg")
}

fn admits(desc: &OpDesc, form: Form) -> bool {
    match desc {
        OpDesc::Reducer(..) => true,
        // a prefix machine has no direct dyadic reading
        OpDesc::Machine(..) => form != Form::Direct,
    }
}

/// Inputs: a CANONICAL spelling and the requested form.  Output: the one descriptor that
/// spelling denotes, or `None` for a registered name and for anything outside the table.
/// Canonical spelling × form is a bijection onto descriptors: every emitter retrieves the same
/// object the resolver checked, so a second table cannot drift into existence.
pub fn canonical(spelling: &str, form: Form) -> Option<OpDesc> {
    let number = |bqn| Some(TypedIdentity { carrier: Carrier::Number, bqn });
    let mask = |bqn| Some(TypedIdentity { carrier: Carrier::Mask, bqn });
    let homogeneous = |step, carrier, identity, associativity, commutativity| {
        OpDesc::Reducer(ReducerDesc {
            spelling: spelling.to_string(),
            input: carrier,
            output: carrier,
            step,
            identity,
            associativity,
            commutativity,
        })
    };
    let machine = |kind, input, state, empty_fold| {
        OpDesc::Machine(PrefixMachineDesc {
            spelling: spelling.to_string(),
            kind,
            input,
            state,
            output: Carrier::Number,
            empty_fold,
        })
    };
    let desc = match spelling {
        "+" => homogeneous("+", Carrier::Number, number("0"), LawStatus::Holds, LawStatus::Holds),
        "-" => homogeneous(
            "-",
            Carrier::Number,
            None,
            LawStatus::DoesNotHold,
            LawStatus::DoesNotHold,
        ),
        "*" => homogeneous("×", Carrier::Number, number("1"), LawStatus::Holds, LawStatus::Holds),
        "/" => homogeneous(
            "÷",
            Carrier::Number,
            None,
            LawStatus::DoesNotHold,
            LawStatus::DoesNotHold,
        ),
        "|" => homogeneous("∨", Carrier::Mask, mask("0"), LawStatus::Holds, LawStatus::Holds),
        "&" => homogeneous("∧", Carrier::Mask, mask("1"), LawStatus::Holds, LawStatus::Holds),
        // finite float64: the numeric extrema carry no identity, never ±∞
        "max" => homogeneous("⌈", Carrier::Number, None, LawStatus::Holds, LawStatus::Holds),
        "min" => homogeneous("⌊", Carrier::Number, None, LawStatus::Holds, LawStatus::Holds),
        "#" => machine(
            MachineKind::Count,
            Carrier::Presence,
            Carrier::Number,
            EmptyFold::Identity(TypedIdentity { carrier: Carrier::Number, bqn: "0" }),
        ),
        "avg" => machine(MachineKind::Average, Carrier::Number, Carrier::Number, EmptyFold::NoRow),
        _ => return None,
    };
    if admits(&desc, form) { Some(desc) } else { None }
}

/// A registered `fn` defaults to a dyadic numeric reducer with undeclared laws and no identity.
/// It advertises fold, scan, and scan-along; declaring instances, arity, or an identity is
/// registry surface this pass does not own.
fn registered(spelling: &str) -> OpDesc {
    OpDesc::Reducer(ReducerDesc {
        spelling: spelling.to_string(),
        input: Carrier::Number,
        output: Carrier::Number,
        step: "",
        identity: None,
        associativity: LawStatus::Undeclared,
        commutativity: LawStatus::Undeclared,
    })
}

/// THE fold/scan head resolver.  Inputs: the surface spelling, the requested form, the inferred
/// operand carrier, and whether the registry answers the name with a `fn`.  Output: the checked
/// descriptor, whose `spelling()` is the canonical head every emitter then retrieves.
pub fn resolve_head(
    spelling: &str,
    form: Form,
    carrier: Carrier,
    registered_fn: bool,
) -> Result<OpDesc, String> {
    let head = match (spelling, carrier) {
        ("+", Carrier::Number)
        | ("-", Carrier::Number)
        | ("*", Carrier::Number)
        | ("/", Carrier::Number)
        | ("|", Carrier::Mask)
        | ("&", Carrier::Mask)
        | ("#", Carrier::Mask)
        | ("#", Carrier::Presence)
        | ("avg", Carrier::Number) => spelling,
        // q/kdb+ Greater and Lesser over the numeric carrier; the bridge names resolve here too
        ("|", Carrier::Number) | ("max", Carrier::Number) => "max",
        ("&", Carrier::Number) | ("min", Carrier::Number) => "min",
        (name, Carrier::Number) if registered_fn => name,
        (name, _) if registered_fn || is_builtin(name) => {
            return Err(format!("reducer '{}' is not defined on {:?}", name, carrier));
        }
        (name, _) => return Err(format!("unknown reducer '{}'", name)),
    };
    let desc = match canonical(head, form) {
        Some(desc) => desc,
        None if registered_fn && !is_builtin(head) => registered(head),
        None => return Err(format!("reducer '{}' has no {} form", spelling, form.name())),
    };
    validate_strategy(&desc, Strategy::ExactLeft)?;
    Ok(desc)
}

/// Validate a descriptor against the execution strategy a plan intends.  Exact left execution
/// admits any type-compatible step; regrouping needs associativity; reordering needs both laws.
/// A prefix machine admits nothing but exact left execution.
pub fn validate_strategy(desc: &OpDesc, strategy: Strategy) -> Result<(), String> {
    if strategy == Strategy::ExactLeft {
        return Ok(());
    }
    let desc = match desc {
        OpDesc::Reducer(desc) => desc,
        OpDesc::Machine(machine) => {
            return Err(format!(
                "'{}' is a prefix machine; only exact left execution is admitted",
                machine.spelling
            ));
        }
    };
    if desc.associativity != LawStatus::Holds {
        return Err(format!(
            "reducer '{}' does not declare associativity; regrouping is not admitted",
            desc.spelling
        ));
    }
    if strategy == Strategy::Reorder && desc.commutativity != LawStatus::Holds {
        return Err(format!(
            "reducer '{}' does not declare commutativity; reordering is not admitted",
            desc.spelling
        ));
    }
    Ok(())
}

/// Convert every backend fold to the ordered left recurrence.  Reducers with a lawful
/// identity are seeded, so empty sums/products/boolean folds retain their language result;
/// extrema remain unseeded and are guarded by the emitter's validity channel.
pub fn left_fold_glyphs(mut bqn: String) -> String {
    let replacements = [
        ("+´", "AnoLeftSum ", "AnoLeftSum ← {+˜´⌽(0∾𝕩)}\n"),
        ("-´", "AnoLeftSubtract ", "AnoLeftSubtract ← {-˜´⌽𝕩}\n"),
        ("×´", "AnoLeftProduct ", "AnoLeftProduct ← {×˜´⌽(1∾𝕩)}\n"),
        ("÷´", "AnoLeftDivide ", "AnoLeftDivide ← {÷˜´⌽𝕩}\n"),
        ("∧´", "AnoLeftAnd ", "AnoLeftAnd ← {∧˜´⌽(1∾𝕩)}\n"),
        ("∨´", "AnoLeftOr ", "AnoLeftOr ← {∨˜´⌽(0∾𝕩)}\n"),
        ("⌈´", "AnoLeftMaximum ", "AnoLeftMaximum ← {⌈˜´⌽𝕩}\n"),
        ("⌊´", "AnoLeftMinimum ", "AnoLeftMinimum ← {⌊˜´⌽𝕩}\n"),
    ];
    let mut declarations = String::new();
    for (from, to, declaration) in replacements {
        if bqn.contains(from) {
            bqn = bqn.replace(from, to);
            declarations.push_str(declaration);
        }
    }
    bqn = left_fold_named(bqn);
    if declarations.is_empty() {
        return bqn;
    }
    let marker = "anoSel ← ⟨⟩\n";
    if let Some(position) = bqn.find(marker) {
        bqn.insert_str(position + marker.len(), &declarations);
    } else {
        bqn.insert_str(0, &declarations);
    }
    bqn
}

fn left_fold_named(input: String) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len() + 32);
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == 'F' && i + 3 < chars.len() && chars[i + 1] == 'n' && chars[i + 2] == '_' {
            let start = i;
            i += 3;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            if i < chars.len() && chars[i] == '´' {
                for c in &chars[start..i] {
                    output.push(*c);
                }
                output.push_str("˜´⌽");
                i += 1;
                continue;
            }
            for c in &chars[start..i] {
                output.push(*c);
            }
            continue;
        }
        output.push(chars[i]);
        i += 1;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(spelling: &str, carrier: Carrier) -> OpDesc {
        resolve_head(spelling, Form::Fold, carrier, false).unwrap()
    }

    #[test]
    fn greater_and_lesser_are_carrier_directed() {
        assert_eq!(head("|", Carrier::Mask).reducer().unwrap().direct_step(), Some("∨"));
        assert_eq!(head("|", Carrier::Number).reducer().unwrap().direct_step(), Some("⌈"));
        assert_eq!(head("&", Carrier::Mask).empty_identity(), Some("1"));
        assert_eq!(head("&", Carrier::Number).empty_identity(), None);
    }

    // Identities are typed: the mask instances carry mask values, the numeric extrema carry none.
    #[test]
    fn identities_are_typed_per_carrier() {
        let and = head("&", Carrier::Mask);
        let or = head("|", Carrier::Mask);
        assert_eq!(
            and.reducer().unwrap().identity,
            Some(TypedIdentity { carrier: Carrier::Mask, bqn: "1" })
        );
        assert_eq!(
            or.reducer().unwrap().identity,
            Some(TypedIdentity { carrier: Carrier::Mask, bqn: "0" })
        );
        assert_eq!(head("max", Carrier::Number).reducer().unwrap().identity, None);
        assert_eq!(head("min", Carrier::Number).reducer().unwrap().identity, None);
        assert_eq!(head("#", Carrier::Presence).empty_identity(), Some("0"));
        assert_eq!(head("avg", Carrier::Number).empty_identity(), None);
    }

    // The bridge spellings are not lookalikes: they resolve to one and the same descriptor.
    #[test]
    fn bridge_spellings_share_one_descriptor() {
        for form in [Form::Fold, Form::Scan, Form::ScanAlong] {
            assert_eq!(
                resolve_head("max", form, Carrier::Number, false),
                resolve_head("|", form, Carrier::Number, false)
            );
            assert_eq!(
                resolve_head("min", form, Carrier::Number, false),
                resolve_head("&", form, Carrier::Number, false)
            );
        }
    }

    #[test]
    fn folds_are_reassociated_to_left_order() {
        assert_eq!(
            left_fold_glyphs("x ← -´v\ny ← Fn_sub´ v\n".to_string()),
            "AnoLeftSubtract ← {-˜´⌽𝕩}\nx ← AnoLeftSubtract v\ny ← Fn_sub˜´⌽ v\n"
        );
        let sum = left_fold_glyphs("anoSel ← ⟨⟩\nx ← +´v\n".to_string());
        assert!(sum.contains("AnoLeftSum ← {+˜´⌽(0∾𝕩)}"));
        assert!(sum.contains("x ← AnoLeftSum v"));
    }

    #[test]
    fn count_and_average_are_prefix_machines() {
        assert_eq!(head("#", Carrier::Presence).machine().unwrap().kind, MachineKind::Count);
        assert_eq!(head("avg", Carrier::Number).machine().unwrap().kind, MachineKind::Average);
        // count consumes selection presence, not a numeric payload
        assert_eq!(head("#", Carrier::Presence).machine().unwrap().input, Carrier::Presence);
        assert!(head("#", Carrier::Presence).fold_glyph().is_none());
    }

    // The carrier gate is the same object for every form, so scan and along agree with fold.
    #[test]
    fn carrier_gate_agrees_across_forms() {
        for form in [Form::Fold, Form::Scan, Form::ScanAlong] {
            for spelling in ["max", "min", "avg", "threat"] {
                let refusal =
                    resolve_head(spelling, form, Carrier::Mask, spelling == "threat").unwrap_err();
                assert_eq!(
                    refusal,
                    format!("reducer '{}' is not defined on Mask", spelling)
                );
            }
            assert_eq!(
                resolve_head("+", form, Carrier::Mask, false).unwrap_err(),
                "reducer '+' is not defined on Mask"
            );
            assert_eq!(
                resolve_head("nope", form, Carrier::Number, false).unwrap_err(),
                "unknown reducer 'nope'"
            );
        }
    }

    // A name is admitted because the registry answers it with a fn, never because it is a name.
    #[test]
    fn registered_names_advertise_every_ordered_form() {
        for form in [Form::Fold, Form::Scan, Form::ScanAlong] {
            let desc = resolve_head("threat", form, Carrier::Number, true).unwrap();
            let reducer = desc.reducer().unwrap();
            assert_eq!(reducer.direct_step(), None);
            assert_eq!(reducer.associativity, LawStatus::Undeclared);
            assert_eq!(desc.empty_identity(), None);
        }
    }

    // Exact left execution admits every step; a regrouping or reordering plan does not.
    #[test]
    fn strategies_refuse_what_they_may_not_assume() {
        let ordered = ["-", "/"];
        for spelling in ordered {
            let desc = head(spelling, Carrier::Number);
            assert!(validate_strategy(&desc, Strategy::ExactLeft).is_ok());
            assert!(validate_strategy(&desc, Strategy::Regroup).is_err());
            assert!(validate_strategy(&desc, Strategy::Reorder).is_err());
        }
        let named = resolve_head("threat", Form::Fold, Carrier::Number, true).unwrap();
        assert!(validate_strategy(&named, Strategy::ExactLeft).is_ok());
        assert!(validate_strategy(&named, Strategy::Regroup)
            .unwrap_err()
            .contains("does not declare associativity"));
        assert!(validate_strategy(&named, Strategy::Reorder).is_err());
        let machine = head("#", Carrier::Presence);
        assert!(validate_strategy(&machine, Strategy::ExactLeft).is_ok());
        assert!(validate_strategy(&machine, Strategy::Regroup)
            .unwrap_err()
            .contains("prefix machine"));
        let sum = head("+", Carrier::Number);
        assert!(validate_strategy(&sum, Strategy::Regroup).is_ok());
        assert!(validate_strategy(&sum, Strategy::Reorder).is_ok());
    }

    // Canonical retrieval is what the emitters use; a machine has no direct dyadic reading.
    #[test]
    fn canonical_retrieval_matches_resolution() {
        assert_eq!(canonical("max", Form::Scan).unwrap().scan_glyph(), Some("⌈`".to_string()));
        assert_eq!(canonical("min", Form::Scan).unwrap().scan_glyph(), Some("⌊`".to_string()));
        assert_eq!(canonical("&", Form::Scan).unwrap().scan_glyph(), Some("∧`".to_string()));
        assert_eq!(canonical("-", Form::Fold).unwrap().fold_glyph(), Some("-´".to_string()));
        assert!(canonical("#", Form::Direct).is_none());
        assert!(canonical("avg", Form::Direct).is_none());
        assert!(canonical("threat", Form::Fold).is_none());
    }
}
