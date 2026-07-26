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
    Char,
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
/// BQN dyadic function — a primitive, or the name of a declared one — and it is empty exactly
/// when a registered function supplies the step.  `step_body` is the dfn a named step stands for
/// and is empty for a primitive: the char instances need one because BQN's `⌈` and `⌊` refuse
/// characters outright, so the char step travels through code points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReducerDesc {
    pub spelling: String,
    pub input: Carrier,
    pub output: Carrier,
    pub step: &'static str,
    pub step_body: &'static str,
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

    /// The declaration `step` names, for the emitter's prologue.  `None` when the step is a BQN
    /// primitive and so needs none.
    pub fn step_declaration(&self) -> Option<String> {
        if self.step_body.is_empty() {
            None
        } else {
            Some(format!("{} ← {}", self.step, self.step_body))
        }
    }

    /// The dfn the direct dyadic instance is spelled by: the step's own body where it has one,
    /// and otherwise the primitive lifted into a dfn.  One table answers `a|b` and `|/a` alike.
    pub fn direct_body(&self) -> Option<String> {
        let step = self.direct_step()?;
        Some(if self.step_body.is_empty() {
            format!("{{𝕨{}𝕩}}", step)
        } else {
            self.step_body.to_string()
        })
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

    pub fn step_declaration(&self) -> Option<String> {
        self.reducer().and_then(ReducerDesc::step_declaration)
    }

    pub fn direct_body(&self) -> Option<String> {
        self.reducer().and_then(ReducerDesc::direct_body)
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
    let homogeneous = |step, body, carrier, identity, associativity, commutativity| {
        OpDesc::Reducer(ReducerDesc {
            spelling: spelling.to_string(),
            input: carrier,
            output: carrier,
            step,
            step_body: body,
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
        "+" => homogeneous("+", "", Carrier::Number, number("0"), LawStatus::Holds, LawStatus::Holds),
        "-" => homogeneous(
            "-",
            "",
            Carrier::Number,
            None,
            LawStatus::DoesNotHold,
            LawStatus::DoesNotHold,
        ),
        "*" => homogeneous("×", "", Carrier::Number, number("1"), LawStatus::Holds, LawStatus::Holds),
        "/" => homogeneous(
            "÷",
            "",
            Carrier::Number,
            None,
            LawStatus::DoesNotHold,
            LawStatus::DoesNotHold,
        ),
        "|" => homogeneous("∨", "", Carrier::Mask, mask("0"), LawStatus::Holds, LawStatus::Holds),
        "&" => homogeneous("∧", "", Carrier::Mask, mask("1"), LawStatus::Holds, LawStatus::Holds),
        // finite float64: the numeric extrema carry no identity, never ±∞
        "max" => homogeneous("⌈", "", Carrier::Number, None, LawStatus::Holds, LawStatus::Holds),
        "min" => homogeneous("⌊", "", Carrier::Number, None, LawStatus::Holds, LawStatus::Holds),
        // A9 over the char carrier: `⌈` and `⌊` refuse characters, so the step steps through code
        // points.  `@` is the null character, `c-@` its code point and `@+n` the character at one.
        // The order is the code-point order, and it has no identity either — there is no greatest
        // or least character to seed an empty fold with, and none is invented.
        "charmax" => homogeneous(
            "AnoCharGreater",
            "{@+(𝕨-@)⌈𝕩-@}",
            Carrier::Char,
            None,
            LawStatus::Holds,
            LawStatus::Holds,
        ),
        "charmin" => homogeneous(
            "AnoCharLesser",
            "{@+(𝕨-@)⌊𝕩-@}",
            Carrier::Char,
            None,
            LawStatus::Holds,
            LawStatus::Holds,
        ),
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
        step_body: "",
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
        // and over the char carrier, by the same A9 derivation and the same bridge names
        ("|", Carrier::Char) | ("max", Carrier::Char) => "charmax",
        ("&", Carrier::Char) | ("min", Carrier::Char) => "charmin",
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

/// The BQN name of the helper that folds one step left.  Keyed on the step and not on the
/// spelling, so the bridge spellings cannot mint two helpers for one operation.  A step with no
/// name here has no fold rendering at all, which refuses at the fold site rather than answering.
fn helper_name(step: &str) -> Option<&'static str> {
    Some(match step {
        "+" => "AnoLeftSum",
        "-" => "AnoLeftSubtract",
        "×" => "AnoLeftProduct",
        "÷" => "AnoLeftDivide",
        "∨" => "AnoLeftOr",
        "∧" => "AnoLeftAnd",
        "⌈" => "AnoLeftMaximum",
        "⌊" => "AnoLeftMinimum",
        "AnoCharGreater" => "AnoLeftCharMaximum",
        "AnoCharLesser" => "AnoLeftCharMinimum",
        _ => return None,
    })
}

/// Render one fold at the point it is lowered.  Inputs: the checked descriptor of the fold head
/// and the already-rendered BQN operand expression.  Output: the BQN call text, paired with the
/// declarations that call depends on, in dependency order, so the emitter can place them in the
/// program prologue.  `None` for the whole pair when the descriptor supplies no step of its own:
/// a prefix machine, or a registered reducer, whose recurrence `render_named_fold` spells because
/// only the emitter knows the generated name.
///
/// An Ano fold is the unseeded left recurrence: the first value of the operand starts the
/// accumulator and the rest apply left to right, so the rendering never rests on BQN's
/// right-to-left reduction order.  A descriptor carrying a lawful identity is seeded with it, and
/// an empty operand then yields that identity.  A descriptor with no identity stays unseeded, and
/// its empty case is answered by the emitter's validity channel, never by a manufactured value.
/// A registered reducer folds through its own generated function name under the same recurrence.
pub fn render_fold(desc: &OpDesc, operand: &str) -> Option<(String, Vec<String>)> {
    let reducer = desc.reducer()?;
    let step = reducer.direct_step()?;
    let name = helper_name(step)?;
    // `F˜´⌽x` is the left recurrence: reversing the operand and swapping the step's arguments
    // turns BQN's right fold into it.  The identity seeds the reversed operand's tail.
    let body = match reducer.empty_identity() {
        Some(identity) => format!("{{{}˜´⌽({}∾𝕩)}}", step, identity),
        None => format!("{{{}˜´⌽𝕩}}", step),
    };
    let mut declarations: Vec<String> = reducer.step_declaration().into_iter().collect();
    declarations.push(format!("{} ← {}", name, body));
    Some((format!("{} {}", name, operand), declarations))
}

/// Fold a registered step under the same left recurrence.  Inputs: the BQN function name the
/// emitter generated for that step and the already-rendered operand.  Output: the BQN call text.
/// No helper declaration: the registered function is declared with the world fixture, and an
/// unseeded registered step has no identity to seed with.
pub fn render_named_fold(function: &str, operand: &str) -> String {
    format!("{}˜´⌽{}", function, operand)
}

/// The BQN name of the mean machine's finish.
const MEAN: &str = "AnoSemAverage";

/// Render the mean machine's finish.  Inputs: none.  Output: the BQN name the emitter applies to
/// a nonempty prefix, paired with the declarations that name depends on in dependency order, for
/// the emitter's prologue.
///
/// The machine's state is `(sum,count)` and it consumes its input in one direction, so the finish
/// is the state after the last element: the same left recurrence every prefix of `avg\` runs.
/// Summing the payload in BQN's own order instead would make `avg/` disagree with the last element
/// of `avg\` over a float column, which is the disagreement A13 exists to remove.  The count is a
/// length, so it carries no order of its own.
pub fn render_mean() -> (&'static str, Vec<String>) {
    let sum = canonical("+", Form::Fold).expect("the numeric sum is a table entry");
    let (call, mut declarations) = render_fold(&sum, "𝕩").expect("the numeric sum renders a fold");
    declarations.push(format!("{} ← {{({})÷≠𝕩}}", MEAN, call));
    (MEAN, declarations)
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
        // A9 over the char carrier is a derivation, not a fourth glyph: one more instance of the
        // same two spellings, with the same two bridges and no identity of its own
        assert_eq!(
            head("|", Carrier::Char).reducer().unwrap().direct_step(),
            Some("AnoCharGreater")
        );
        assert_eq!(
            head("&", Carrier::Char).reducer().unwrap().direct_step(),
            Some("AnoCharLesser")
        );
        assert_eq!(head("|", Carrier::Char).empty_identity(), None);
        assert_eq!(head("&", Carrier::Char).empty_identity(), None);
        assert_eq!(head("max", Carrier::Char), head("|", Carrier::Char));
        assert_eq!(head("min", Carrier::Char), head("&", Carrier::Char));
    }

    // The char step is a declared function because BQN's `⌈`/`⌊` refuse characters; the direct
    // dyad, the fold and the scan all spell that one step, so they cannot diverge.
    #[test]
    fn the_char_step_travels_through_code_points() {
        let greater = head("|", Carrier::Char);
        assert_eq!(
            greater.step_declaration().unwrap(),
            "AnoCharGreater ← {@+(𝕨-@)⌈𝕩-@}"
        );
        assert_eq!(greater.direct_body().unwrap(), "{@+(𝕨-@)⌈𝕩-@}");
        assert_eq!(
            canonical("charmax", Form::Scan).unwrap().scan_glyph(),
            Some("AnoCharGreater`".to_string())
        );
        assert_eq!(
            canonical("charmin", Form::Scan).unwrap().scan_glyph(),
            Some("AnoCharLesser`".to_string())
        );
        let (call, declarations) = render_fold(&greater, "glyph").unwrap();
        assert_eq!(call, "AnoLeftCharMaximum glyph");
        assert_eq!(
            declarations,
            vec![
                "AnoCharGreater ← {@+(𝕨-@)⌈𝕩-@}".to_string(),
                "AnoLeftCharMaximum ← {AnoCharGreater˜´⌽𝕩}".to_string(),
            ]
        );
        // a primitive step needs no declaration of its own, and lifts into the same dyad shape
        assert_eq!(head("|", Carrier::Number).step_declaration(), None);
        assert_eq!(head("|", Carrier::Number).direct_body().unwrap(), "{𝕨⌈𝕩}");
        assert_eq!(head("&", Carrier::Mask).direct_body().unwrap(), "{𝕨∧𝕩}");
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

    // The ruled recurrence at the point of rendering: reversal and argument swap, seeded only
    // where a lawful identity exists.  `-´1‿2‿3` is 2 in BQN and ¯4 in Ano; this is the seam.
    #[test]
    fn folds_render_the_ordered_left_recurrence() {
        let (call, declarations) = render_fold(&head("-", Carrier::Number), "gold").unwrap();
        assert_eq!(call, "AnoLeftSubtract gold");
        assert_eq!(declarations, vec!["AnoLeftSubtract ← {-˜´⌽𝕩}".to_string()]);
        let (call, declarations) = render_fold(&head("+", Carrier::Number), "(path/gold)").unwrap();
        assert_eq!(call, "AnoLeftSum (path/gold)");
        assert_eq!(declarations, vec!["AnoLeftSum ← {+˜´⌽(0∾𝕩)}".to_string()]);
        // the numeric extrema declare no identity, so nothing is seeded and no ∞ is minted
        let (_, declarations) = render_fold(&head("max", Carrier::Number), "gold").unwrap();
        assert_eq!(declarations, vec!["AnoLeftMaximum ← {⌈˜´⌽𝕩}".to_string()]);
        // one descriptor, one helper: the bridge spellings cannot render two
        assert_eq!(
            render_fold(&head("|", Carrier::Number), "gold"),
            render_fold(&head("max", Carrier::Number), "gold")
        );
        // a prefix machine and a registered step supply no step of their own
        assert!(render_fold(&head("#", Carrier::Presence), "gold").is_none());
        let named = resolve_head("threat", Form::Fold, Carrier::Number, true).unwrap();
        assert!(render_fold(&named, "gold").is_none());
        assert_eq!(render_named_fold("Fn_threat", "𝕩"), "Fn_threat˜´⌽𝕩");
    }

    // Every step the table carries renders a fold, so the helper names cannot drift out from
    // under the descriptors that key them.
    #[test]
    fn every_direct_step_renders_a_fold() {
        for spelling in ["+", "-", "*", "/", "|", "&", "max", "min"] {
            for carrier in [Carrier::Number, Carrier::Mask, Carrier::Char] {
                let Ok(desc) = resolve_head(spelling, Form::Fold, carrier, false) else { continue };
                assert!(render_fold(&desc, "x").is_some(), "{} over {:?}", spelling, carrier);
            }
        }
    }

    // The mean's finish accumulates in the machine's order, not BQN's: it is the last prefix of
    // `avg\`, and over 1‿1e100‿¯1e100 the two orders answer 0 and 1.
    #[test]
    fn the_mean_finishes_on_its_own_ordered_sum() {
        let (name, declarations) = render_mean();
        assert_eq!(name, "AnoSemAverage");
        assert_eq!(
            declarations,
            vec![
                "AnoLeftSum ← {+˜´⌽(0∾𝕩)}".to_string(),
                "AnoSemAverage ← {(AnoLeftSum 𝕩)÷≠𝕩}".to_string(),
            ]
        );
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
            // the char carrier admits Greater/Lesser and their bridges, and nothing else
            for spelling in ["+", "-", "*", "/", "avg", "threat"] {
                assert_eq!(
                    resolve_head(spelling, form, Carrier::Char, spelling == "threat").unwrap_err(),
                    format!("reducer '{}' is not defined on Char", spelling)
                );
            }
            assert_eq!(
                resolve_head("nope", form, Carrier::Number, false).unwrap_err(),
                "unknown reducer 'nope'"
            );
            // the canonical char spellings are not a surface: nothing admits them as written
            for spelling in ["charmax", "charmin"] {
                assert_eq!(
                    resolve_head(spelling, form, Carrier::Char, false).unwrap_err(),
                    format!("unknown reducer '{}'", spelling)
                );
            }
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
