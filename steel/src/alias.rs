//! Dynamic `^name` aliases.
//!
//! Static registry aliases (`as`/`ja`) remain spelling aliases.  This module owns the
//! separately-persisted live overlay used only by `NodeKind::Alias`.  Steel loads one
//! immutable snapshot at the beginning of an emission, so every statement in that emission
//! observes one coherent alias state.
//!
//! Determinism boundary, declared structurally: a resolver is `fn(&Registry, &HostInput) ->
//! Result<ResolverValue, Diag>` — a plain fn pointer over the frozen world and the frozen
//! host-input snapshot recorded on its target.  No other input is representable, so a resolver
//! cannot observe ambient state, and rerunning one against the same frozen pair is
//! observationally identical.  A resolver that needed more would have to widen this type, which
//! is the visible design event.

use crate::num;
use crate::registry::reg_find;
use crate::{
    ArrayDomain, BindKind, CallableSignature, ColType, ConstructorRefinement, DeclId, DeclMeta,
    Diag, RegEntryKind, RegType, Registry, ServiceDirection,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const MAGIC: &str = "ano-aliases-v1";

fn fail(message: impl Into<String>) -> Diag {
    Diag::refuse(format!("alias: {}", message.into()))
}

/// Dynamic aliases live beside the registry, without changing the registry grammar.
pub fn sidecar_path(registry_path: &str) -> PathBuf {
    PathBuf::from(format!("{}.aliases", registry_path))
}

/// The registry name fold: ASCII A-Z only, all non-ASCII bytes exact.
pub fn canonical_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c })
        .collect()
}

/// The two resolver requests a source name can make.
/// `Bare` is `name`: registry/def lookup only, never the overlay.
/// `DynamicAliasThenBare` is `^name`: overlay first, explicit bare fallback (A4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupMode {
    Bare,
    DynamicAliasThenBare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasCarrier {
    Mask,
    Number,
    Entity,
    Point,
    Vector,
    Symbol,
}

impl AliasCarrier {
    pub fn word(self) -> &'static str {
        match self {
            AliasCarrier::Mask => "mask",
            AliasCarrier::Number => "number",
            AliasCarrier::Entity => "entity",
            AliasCarrier::Point => "point",
            AliasCarrier::Vector => "vector",
            AliasCarrier::Symbol => "symbol",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "mask" => AliasCarrier::Mask,
            "number" => AliasCarrier::Number,
            "entity" => AliasCarrier::Entity,
            "point" => AliasCarrier::Point,
            "vector" => AliasCarrier::Vector,
            "symbol" => AliasCarrier::Symbol,
            _ => return None,
        })
    }
}

/// The frozen host-input events one resolver target was installed against.  `version` is the
/// overlay version at which the snapshot became visible; `pairs` is the whole world the resolver
/// may read besides the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct HostInput {
    pub version: u64,
    pub pairs: BTreeMap<String, String>,
}

/// What a resolver may return.  Both shapes carry their own row domain, checked against the
/// declared carrier at install, validate, and resolve.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolverValue {
    Entity(usize),
    Mask(Vec<f64>),
}

/// A registered host resolver.  `service` versions the behaviour: a target records the service
/// it was installed against and refuses when the registered service moves under it.
#[derive(Debug, Clone, Copy)]
pub struct ResolverSpec {
    pub id: &'static str,
    pub service: u64,
    pub carrier: AliasCarrier,
    pub run: fn(&Registry, &HostInput) -> Result<ResolverValue, Diag>,
}

// Inputs: the frozen registry and host input. Output: the entity row named by pairs["entity"].
// The deictic shape: a host-frozen entity referent, refused when it leaves the world.
fn run_entity_input(
    reg: &Registry,
    input: &HostInput,
    resolver: &str,
    key: &str,
) -> Result<ResolverValue, Diag> {
    let Some(spelling) = input.pairs.get(key) else {
        return Err(fail(format!("resolver '{}' needs a '{}' input", resolver, key)));
    };
    let Ok(row) = spelling.parse::<usize>() else {
        return Err(fail(format!(
            "resolver '{}' input '{}' is not a row index",
            resolver, spelling
        )));
    };
    let rows = reg.n.max(0) as usize;
    if row >= rows {
        return Err(fail(format!(
            "resolver '{}' row {} is outside a world of {} rows",
            resolver, row, rows
        )));
    }
    Ok(ResolverValue::Entity(row))
}

fn run_input_entity(reg: &Registry, input: &HostInput) -> Result<ResolverValue, Diag> {
    run_entity_input(reg, input, "input.entity", "entity")
}

// Inputs: the frozen registry and host input. Output: the mask spelled by pairs["mask"] as
// whitespace-separated 0/1 tokens, exactly one per registry row.
fn run_mask_input(
    reg: &Registry,
    input: &HostInput,
    resolver: &str,
    key: &str,
) -> Result<ResolverValue, Diag> {
    let Some(spelling) = input.pairs.get(key) else {
        return Err(fail(format!("resolver '{}' needs a '{}' input", resolver, key)));
    };
    let rows = reg.n.max(0) as usize;
    let mut values = Vec::with_capacity(rows);
    for token in spelling.split_whitespace() {
        values.push(match token {
            "0" => 0.0,
            "1" => 1.0,
            _ => {
                return Err(fail(format!(
                    "resolver '{}' token '{}' is not 0 or 1",
                    resolver, token
                )))
            }
        });
    }
    if values.len() != rows {
        return Err(fail(format!(
            "resolver '{}' produced {} rows, registry has {}",
            resolver, values.len(), rows
        )));
    }
    Ok(ResolverValue::Mask(values))
}

fn run_input_mask(reg: &Registry, input: &HostInput) -> Result<ResolverValue, Diag> {
    run_mask_input(reg, input, "input.mask", "mask")
}

// Concrete deictics. The host freezes these named inputs at the statement barrier; the
// resolver table fixes each result carrier and service version. The generic input resolvers
// remain useful to embedders, while these four give the language's documented pronouns one
// stable host contract instead of asking each integration to invent an id.
fn run_cursor(reg: &Registry, input: &HostInput) -> Result<ResolverValue, Diag> {
    run_entity_input(reg, input, "deictic.cursor", "cursor")
}

fn run_observer(reg: &Registry, input: &HostInput) -> Result<ResolverValue, Diag> {
    run_entity_input(reg, input, "deictic.observer", "observer")
}

fn run_selected(reg: &Registry, input: &HostInput) -> Result<ResolverValue, Diag> {
    run_mask_input(reg, input, "deictic.selected", "selected")
}

fn run_world(reg: &Registry, _input: &HostInput) -> Result<ResolverValue, Diag> {
    Ok(ResolverValue::Mask(vec![1.0; reg.n.max(0) as usize]))
}

const RESOLVERS: &[ResolverSpec] = &[
    ResolverSpec {
        id: "input.entity",
        service: 1,
        carrier: AliasCarrier::Entity,
        run: run_input_entity,
    },
    ResolverSpec {
        id: "input.mask",
        service: 1,
        carrier: AliasCarrier::Mask,
        run: run_input_mask,
    },
    ResolverSpec {
        id: "deictic.cursor",
        service: 1,
        carrier: AliasCarrier::Entity,
        run: run_cursor,
    },
    ResolverSpec {
        id: "deictic.observer",
        service: 1,
        carrier: AliasCarrier::Entity,
        run: run_observer,
    },
    ResolverSpec {
        id: "deictic.selected",
        service: 1,
        carrier: AliasCarrier::Mask,
        run: run_selected,
    },
    ResolverSpec {
        id: "deictic.world",
        service: 1,
        carrier: AliasCarrier::Mask,
        run: run_world,
    },
];

/// The registered resolver under `id`, or `None`.  The table is const: registration is a
/// compile-time act, never host data.
pub fn resolver_spec(id: &str) -> Option<&'static ResolverSpec> {
    RESOLVERS.iter().find(|spec| spec.id == id)
}
#[derive(Debug, Clone, Copy)]
struct ResolverContract {
    declaration: Option<DeclId>,
    service: u64,
    carrier: AliasCarrier,
}

// A matching service declaration upgrades the legacy compile-time resolver contract. Its
// signature is closed: frozen host input is out-of-band, so the service is unit -> entity|mask.
fn resolver_contract(
    reg: &Registry,
    spec: &ResolverSpec,
) -> Result<ResolverContract, Diag> {
    let Some(index) = reg_find(reg, spec.id) else {
        return Ok(ResolverContract {
            declaration: None,
            service: spec.service,
            carrier: spec.carrier,
        });
    };
    let RegEntryKind::Service { descriptor } = &reg.ents[index].kind else {
        return Err(fail(format!(
            "resolver '{}' is shadowed by a non-service declaration",
            spec.id
        )));
    };
    if descriptor.direction != ServiceDirection::Input || !descriptor.signature.inputs.is_empty() {
        return Err(fail(format!(
            "resolver '{}' needs an input service with signature unit->{}",
            spec.id,
            spec.carrier.word()
        )));
    }
    let carrier = match descriptor.signature.output {
        RegType::Entity => AliasCarrier::Entity,
        RegType::Mask => AliasCarrier::Mask,
        _ => {
            return Err(fail(format!(
                "resolver '{}' service output is not entity or mask",
                spec.id
            )));
        }
    };
    if carrier != spec.carrier {
        return Err(fail(format!(
            "resolver '{}' service declares {}, implementation returns {}",
            spec.id,
            carrier.word(),
            spec.carrier.word()
        )));
    }
    Ok(ResolverContract {
        declaration: Some(descriptor.meta.id),
        service: descriptor.meta.version,
        carrier,
    })
}


// Inputs: a resolver result, the carrier its target declared, the frozen registry. Output: true
// when the result inhabits that carrier and row domain — Entity < n, Mask exactly n rows of 0/1.
fn value_fits(value: &ResolverValue, carrier: AliasCarrier, reg: &Registry) -> bool {
    let rows = reg.n.max(0) as usize;
    match (value, carrier) {
        (ResolverValue::Entity(row), AliasCarrier::Entity) => *row < rows,
        (ResolverValue::Mask(values), AliasCarrier::Mask) => {
            values.len() == rows && values.iter().all(|v| *v == 0.0 || *v == 1.0)
        }
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AliasTarget {
    Binding {
        binding: String,
        carrier: AliasCarrier,
        schema: u64,
    },
    Mask {
        values: Vec<f64>,
        rows: usize,
        schema: u64,
    },
    Resolver {
        resolver: String,
        carrier: AliasCarrier,
        service: u64,
        service_declaration: Option<DeclId>,
        schema: u64,
        input: HostInput,
    },
}

fn service_description(version: u64, declaration: Option<DeclId>) -> String {
    match declaration {
        Some(id) => format!("{} declaration {:016x}", version, id.0),
        None => version.to_string(),
    }
}

impl AliasTarget {
    pub fn carrier(&self) -> AliasCarrier {
        match self {
            AliasTarget::Binding { carrier, .. } => *carrier,
            AliasTarget::Mask { .. } => AliasCarrier::Mask,
            AliasTarget::Resolver { carrier, .. } => *carrier,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            AliasTarget::Binding { binding, carrier, .. } => {
                format!("{} ({})", binding, carrier.word())
            }
            AliasTarget::Mask { values, .. } => {
                let bits = values
                    .iter()
                    .map(|v| if *v == 0.0 { "0" } else { "1" })
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("mask [{}]", bits)
            }
            AliasTarget::Resolver { resolver, carrier, service, service_declaration, input, .. } => format!(
                "resolver {} ({}, service {}, input v{})",
                resolver,
                carrier.word(),
                service_description(*service, *service_declaration),
                input.version
            ),
        }
    }

    /// Inputs: the synthetic registry name a materialized result took, when there is one.
    /// Output: the trace clause naming what answered one consultation.
    pub fn provenance(&self, materialized: Option<&str>) -> String {
        match self {
            AliasTarget::Binding { binding, .. } => format!("binding '{}'", binding),
            AliasTarget::Mask { .. } => {
                format!("mask '{}'", materialized.unwrap_or(""))
            }
            AliasTarget::Resolver { resolver, service, service_declaration, input, .. } => format!(
                "resolver '{}' service {} input v{}",
                resolver, service_description(*service, *service_declaration), input.version
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasEnvironment {
    version: u64,
    schema: u64,
    entries: BTreeMap<String, AliasTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasSnapshot {
    pub version: u64,
    pub schema: u64,
    entries: BTreeMap<String, AliasTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedAlias {
    Entry(usize),
    Mask(Vec<f64>),
    EntityRow(usize),
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hash_field(hash: u64, bytes: &[u8]) -> u64 {
    let hash = hash_bytes(hash, &(bytes.len() as u64).to_le_bytes());
    hash_bytes(hash, bytes)
}

fn hash_text(hash: u64, text: &str) -> u64 {
    hash_field(hash, text.as_bytes())
}

fn hash_optional_text(mut hash: u64, text: &Option<String>) -> u64 {
    hash = hash_bytes(hash, &[text.is_some() as u8]);
    if let Some(text) = text {
        hash = hash_text(hash, text);
    }
    hash
}

fn hash_number(hash: u64, value: f64) -> u64 {
    let bits = if value == 0.0 { 0 } else { value.to_bits() };
    hash_bytes(hash, &bits.to_le_bytes())
}

fn hash_range(mut hash: u64, range: Option<(f64, f64)>) -> u64 {
    hash = hash_bytes(hash, &[range.is_some() as u8]);
    if let Some((lo, hi)) = range {
        hash = hash_number(hash, lo);
        hash = hash_number(hash, hi);
    }
    hash
}

fn hash_meta(mut hash: u64, meta: DeclMeta) -> u64 {
    hash = hash_bytes(hash, &meta.id.0.to_le_bytes());
    hash_bytes(hash, &meta.version.to_le_bytes())
}

fn hash_reg_type(mut hash: u64, carrier: &RegType) -> u64 {
    let tag = match carrier {
        RegType::Unit => 0,
        RegType::Mask => 1,
        RegType::Nat => 2,
        RegType::Int => 3,
        RegType::Num => 4,
        RegType::Sym => 5,
        RegType::Char => 6,
        RegType::Entity => 7,
        RegType::Named(_) => 8,
    };
    hash = hash_bytes(hash, &[tag]);
    if let RegType::Named(name) = carrier {
        hash = hash_text(hash, name);
    }
    hash
}

fn hash_signature(mut hash: u64, signature: &CallableSignature) -> u64 {
    hash = hash_bytes(hash, &(signature.inputs.len() as u64).to_le_bytes());
    for input in &signature.inputs {
        hash = hash_reg_type(hash, input);
    }
    hash_reg_type(hash, &signature.output)
}

fn hash_names(mut hash: u64, names: &[String]) -> u64 {
    hash = hash_bytes(hash, &(names.len() as u64).to_le_bytes());
    for name in names {
        hash = hash_text(hash, name);
    }
    hash
}

/// Structural fingerprint.  World values and `n` deliberately do not participate: ordinary
/// entity updates do not stale binding aliases.  Materialized mask aliases independently pin
/// their row count.  Every declaration descriptor does participate: this hash is also the
/// persistent schema identity used by migration manifests and stamped handles.
/// The retired inferred-pair byte remains in the wire hash as canonical zero.
pub fn registry_fingerprint(reg: &Registry) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    hash = hash_bytes(hash, b"ano-schema-fingerprint-v2");
    hash = hash_bytes(hash, &reg.lat_w.to_le_bytes());
    hash = hash_bytes(hash, &reg.lat_h.to_le_bytes());
    for entry in &reg.ents {
        hash = hash_text(hash, &entry.name);
        hash = hash_number(hash, entry.defval);
        match &entry.kind {
            RegEntryKind::Col { ty, uniq, pres, rng, .. } => {
                hash = hash_bytes(hash, &[0x10, *ty as u8, *uniq as u8, pres.is_some() as u8]);
                hash = hash_bytes(hash, &[0]);
                hash = hash_range(hash, *rng);
            }
            RegEntryKind::Field { ty, rng, .. } => {
                hash = hash_bytes(hash, &[0x20, *ty as u8]);
                hash = hash_bytes(hash, &[0]);
                hash = hash_range(hash, *rng);
            }
            RegEntryKind::Rel { key_of, .. } => {
                hash = hash_bytes(hash, &[0x30]);
                hash = hash_optional_text(hash, key_of);
            }
            RegEntryKind::SRel { inv_of, key_of, .. } => {
                hash = hash_bytes(hash, &[0x40]);
                hash = hash_optional_text(hash, inv_of);
                hash = hash_optional_text(hash, key_of);
            }
            RegEntryKind::AliasMask { .. } => {
                hash = hash_bytes(hash, &[0x50]);
            }
            RegEntryKind::Bind { kind, .. } => {
                hash = hash_bytes(hash, &[0x60, *kind as u8]);
            }
            RegEntryKind::Fn { body } => {
                hash = hash_bytes(hash, &[0x70]);
                hash = hash_optional_text(hash, body);
            }
            RegEntryKind::TypedFn { body, descriptor } => {
                hash = hash_bytes(hash, &[0x71]);
                hash = hash_meta(hash, descriptor.meta);
                hash = hash_signature(hash, &descriptor.signature);
                hash = hash_bytes(
                    hash,
                    &[
                        descriptor.effects.read as u8,
                        descriptor.effects.write as u8,
                        descriptor.effects.service as u8,
                        descriptor.determinism as u8,
                        descriptor.trust as u8,
                    ],
                );
                hash = hash_names(hash, &descriptor.reads);
                hash = hash_names(hash, &descriptor.writes);
                hash = hash_names(hash, &descriptor.services);
                hash = hash_text(hash, body);
            }
            RegEntryKind::Array { descriptor } => {
                hash = hash_bytes(hash, &[0xa0]);
                hash = hash_meta(hash, descriptor.meta);
                hash = hash_reg_type(hash, &descriptor.carrier);
                match descriptor.domain {
                    ArrayDomain::Scalar => hash = hash_bytes(hash, &[0]),
                    ArrayDomain::Entity => hash = hash_bytes(hash, &[1]),
                    ArrayDomain::Fixed(extent) => {
                        hash = hash_bytes(hash, &[2]);
                        hash = hash_bytes(hash, &extent.to_le_bytes());
                    }
                }
            }
            RegEntryKind::Service { descriptor } => {
                hash = hash_bytes(hash, &[0xa1]);
                hash = hash_meta(hash, descriptor.meta);
                hash = hash_bytes(hash, &[descriptor.direction as u8, descriptor.trust as u8]);
                hash = hash_signature(hash, &descriptor.signature);
            }
            RegEntryKind::Enum { descriptor } => {
                hash = hash_bytes(hash, &[0xa2]);
                hash = hash_meta(hash, descriptor.meta);
                hash = hash_bytes(hash, &(descriptor.cases.len() as u64).to_le_bytes());
                for case in &descriptor.cases {
                    hash = hash_text(hash, &case.name);
                    hash = hash_bytes(hash, &case.discriminant.to_le_bytes());
                }
                hash = hash_bytes(hash, &(descriptor.reserved.len() as u64).to_le_bytes());
                for discriminant in &descriptor.reserved {
                    hash = hash_bytes(hash, &discriminant.to_le_bytes());
                }
            }
            RegEntryKind::Ctor { descriptor } => {
                hash = hash_bytes(hash, &[0xa3]);
                hash = hash_meta(hash, descriptor.meta);
                match &descriptor.refinement {
                    ConstructorRefinement::Range { lo, hi } => {
                        hash = hash_bytes(hash, &[0]);
                        hash = hash_number(hash, *lo);
                        hash = hash_number(hash, *hi);
                    }
                    ConstructorRefinement::Enum { enumeration } => {
                        hash = hash_bytes(hash, &[1]);
                        hash = hash_text(hash, enumeration);
                    }
                }
            }
            RegEntryKind::Tag { col, carrier_ty, num, sym } => {
                hash = hash_bytes(hash, &[0x80, *carrier_ty as u8]);
                hash = hash_text(hash, col);
                hash = hash_number(hash, *num);
                hash = hash_optional_text(hash, sym);
            }
            RegEntryKind::Proto { fields } => {
                hash = hash_bytes(hash, &[0x90]);
                hash = hash_bytes(hash, &(fields.len() as u64).to_le_bytes());
                for field in fields {
                    hash = hash_text(hash, &field.col);
                    hash = hash_text(hash, &field.spelling);
                    hash = hash_number(hash, field.num);
                }
            }
        }
        hash = hash_bytes(hash, &[0xff]);
    }
    hash = hash_bytes(hash, &(reg.aliases.len() as u64).to_le_bytes());
    for row in &reg.aliases {
        hash = hash_text(hash, &row.from);
        hash = hash_text(hash, &row.to);
        hash = hash_bytes(hash, &[row.ja as u8]);
    }
    hash = hash_bytes(hash, &(reg.roles.len() as u64).to_le_bytes());
    for (role, column) in &reg.roles {
        hash = hash_text(hash, role);
        hash = hash_text(hash, column);
    }
    hash_bytes(
        hash,
        &[match reg.reap {
            None => 0,
            Some(crate::Reap::Seal) => 1,
            Some(crate::Reap::Host) => 2,
        }],
    )
}

fn entry_carrier(reg: &Registry, index: usize) -> Result<AliasCarrier, Diag> {
    let Some(entry) = reg.ents.get(index) else {
        return Err(fail(format!("registry entry {} is stale", index)));
    };
    Ok(match &entry.kind {
        RegEntryKind::Col { ty: ColType::Bool, .. }
        | RegEntryKind::Field { ty: ColType::Bool, .. }
        | RegEntryKind::AliasMask { .. }
        | RegEntryKind::Tag { .. } => AliasCarrier::Mask,
        RegEntryKind::Col { ty: ColType::Sym | ColType::Char, .. }
        | RegEntryKind::Field { ty: ColType::Sym | ColType::Char, .. } => AliasCarrier::Symbol,
        RegEntryKind::Col { .. }
        | RegEntryKind::Field { .. }
        | RegEntryKind::Rel { .. } => AliasCarrier::Number,
        RegEntryKind::SRel { .. } => AliasCarrier::Vector,
        RegEntryKind::Bind { kind, .. } => match kind {
            BindKind::Entity => AliasCarrier::Entity,
            BindKind::Mask => AliasCarrier::Mask,
            BindKind::Point => AliasCarrier::Point,
            BindKind::Num => AliasCarrier::Number,
            BindKind::Vec => AliasCarrier::Vector,
        },
        RegEntryKind::Fn { .. }
        | RegEntryKind::TypedFn { .. }
        | RegEntryKind::Array { .. }
        | RegEntryKind::Service { .. }
        | RegEntryKind::Enum { .. }
        | RegEntryKind::Ctor { .. }
        | RegEntryKind::Proto { .. } => {
            return Err(fail(format!("'{}' is not a value binding", entry.name)));
        }
    })
}

fn validate_name(name: &str, what: &str) -> Result<(), Diag> {
    if name.is_empty() || name.bytes().any(|b| matches!(b, b'\t' | b'\r' | b'\n')) {
        return Err(fail(format!("{} must be a nonempty single-line token", what)));
    }
    Ok(())
}

// Inputs: the host-supplied input members. Output: the frozen map, or a refusal. Duplicate keys
// are a host error, not a last-writer-wins merge; keys and values follow the name shape.
fn input_pairs(pairs: &[(String, String)]) -> Result<BTreeMap<String, String>, Diag> {
    let mut map = BTreeMap::new();
    for (key, value) in pairs {
        validate_name(key, "resolver input key")?;
        if key.contains('=') {
            return Err(fail("resolver input key cannot contain '='"));
        }
        validate_name(value, "resolver input value")?;
        if map.insert(key.clone(), value.clone()).is_some() {
            return Err(fail(format!("duplicate resolver input '{}'", key)));
        }
    }
    Ok(map)
}

fn service_stamp_word(declaration: Option<DeclId>, version: u64) -> String {
    match declaration {
        Some(id) => format!("{:016x}@{}", id.0, version),
        None => version.to_string(),
    }
}

fn parse_service_stamp(word: &str) -> Option<(Option<DeclId>, u64)> {
    let (declaration, version_word) = if let Some((id_word, version_word)) = word.split_once('@') {
        if id_word.len() != 16
            || !id_word.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return None;
        }
        let id = u64::from_str_radix(id_word, 16).ok()?;
        if id == 0 {
            return None;
        }
        (Some(DeclId(id)), version_word)
    } else {
        (None, word)
    };
    let version = version_word.parse::<u64>().ok()?;
    if version == 0 || version_word != version.to_string() {
        return None;
    }
    Some((declaration, version))
}

fn binding_target(reg: &Registry, binding: &str, schema: u64) -> Result<AliasTarget, Diag> {
    let Some(index) = reg_find(reg, binding) else {
        return Err(fail(format!("unregistered binding '{}'", binding)));
    };
    Ok(AliasTarget::Binding {
        binding: reg.ents[index].name.clone(),
        carrier: entry_carrier(reg, index)?,
        schema,
    })
}

impl AliasEnvironment {
    pub fn for_registry(reg: &Registry) -> Self {
        Self { version: 0, schema: registry_fingerprint(reg), entries: BTreeMap::new() }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn schema(&self) -> u64 {
        self.schema
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &AliasTarget)> {
        self.entries.iter().map(|(name, target)| (name.as_str(), target))
    }

    fn check_schema(&self, reg: &Registry) -> Result<u64, Diag> {
        let current = registry_fingerprint(reg);
        if self.schema != 0 && self.schema != current {
            return Err(fail(format!(
                "environment schema {:016x} is stale for registry {:016x}",
                self.schema, current
            )));
        }
        Ok(current)
    }

    pub fn snapshot(&self, reg: &Registry) -> Result<AliasSnapshot, Diag> {
        let schema = self.check_schema(reg)?;
        let snapshot = AliasSnapshot {
            version: self.version,
            schema,
            entries: self.entries.clone(),
        };
        snapshot.validate(reg)?;
        Ok(snapshot)
    }

    pub fn install_binding(
        &mut self,
        reg: &Registry,
        name: &str,
        binding: &str,
    ) -> Result<(), Diag> {
        validate_name(name, "alias name")?;
        let schema = self.check_schema(reg)?;
        let target = binding_target(reg, binding, schema)?;
        self.schema = schema;
        self.version = self.version.wrapping_add(1);
        self.entries.insert(canonical_name(name), target);
        Ok(())
    }

    pub fn install_mask(
        &mut self,
        reg: &Registry,
        name: &str,
        values: &[f64],
    ) -> Result<(), Diag> {
        validate_name(name, "alias name")?;
        let schema = self.check_schema(reg)?;
        let rows = reg.n.max(0) as usize;
        if values.len() != rows {
            return Err(fail(format!(
                "mask '^{}' has {} rows, registry has {}",
                name,
                values.len(),
                rows
            )));
        }
        if let Some((row, value)) = values
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value != 0.0 && *value != 1.0)
        {
            return Err(fail(format!(
                "mask '^{}' row {} is {}, expected 0 or 1",
                name,
                row,
                num::fmt_g(17, value)
            )));
        }
        self.schema = schema;
        self.version = self.version.wrapping_add(1);
        self.entries.insert(
            canonical_name(name),
            AliasTarget::Mask { values: values.to_vec(), rows, schema },
        );
        Ok(())
    }

    // Inputs: the frozen registry, the alias stem, a registered resolver id, the host input
    // members. Output: Ok after the target installed. The resolver runs once here against the
    // frozen (registry, input) pair and its result must inhabit the DECLARED carrier and row
    // domain; nothing is written until every check passes.
    pub fn install_resolver(
        &mut self,
        reg: &Registry,
        name: &str,
        id: &str,
        pairs: &[(String, String)],
    ) -> Result<(), Diag> {
        validate_name(name, "alias name")?;
        validate_name(id, "resolver id")?;
        let members = input_pairs(pairs)?;
        let schema = self.check_schema(reg)?;
        let Some(spec) = resolver_spec(id) else {
            return Err(fail(format!("unknown resolver '{}'", id)));
        };
        let contract = resolver_contract(reg, spec)?;
        let input = HostInput { version: self.version.wrapping_add(1), pairs: members };
        let value = (spec.run)(reg, &input)?;
        if !value_fits(&value, contract.carrier, reg) {
            return Err(fail(format!(
                "resolver '{}' result does not inhabit {} over {} rows",
                id,
                contract.carrier.word(),
                reg.n.max(0)
            )));
        }
        self.schema = schema;
        self.version = self.version.wrapping_add(1);
        self.entries.insert(
            canonical_name(name),
            AliasTarget::Resolver {
                resolver: spec.id.to_string(),
                carrier: contract.carrier,
                service: contract.service,
                service_declaration: contract.declaration,
                schema,
                input,
            },
        );
        Ok(())
    }

    pub fn delete(&mut self, name: &str) -> bool {
        if self.entries.remove(&canonical_name(name)).is_none() {
            return false;
        }
        self.version = self.version.wrapping_add(1);
        true
    }

    pub fn clear(&mut self, reg: &Registry) {
        self.entries.clear();
        self.schema = registry_fingerprint(reg);
        self.version = self.version.wrapping_add(1);
    }

    // Inputs: the old/new registries, the migration's canonical old-binding -> new-binding map,
    // and explicitly removed overlay stems. Output: one revalidated environment transition. A
    // missing target, carrier drift, service drift, or stale resolver refuses the whole migration.
    pub fn migrate_schema(
        &self,
        old: &Registry,
        new: &Registry,
        bindings: &BTreeMap<String, String>,
        removed: &BTreeSet<String>,
    ) -> Result<Self, Diag> {
        self.snapshot(old)?;
        let schema = registry_fingerprint(new);
        let mut entries = BTreeMap::new();
        for (name, target) in &self.entries {
            if removed.contains(name) {
                continue;
            }
            let migrated = match target {
                AliasTarget::Binding { binding, carrier, .. } => {
                    let Some(new_binding) = bindings.get(&canonical_name(binding)) else {
                        return Err(fail(format!(
                            "'^{}' targets removed binding '{}'",
                            name, binding
                        )));
                    };
                    let migrated = binding_target(new, new_binding, schema)?;
                    if migrated.carrier() != *carrier {
                        return Err(fail(format!("'^{}' binding carrier changed", name)));
                    }
                    migrated
                }
                AliasTarget::Mask { values, rows, .. } => {
                    if *rows != new.n.max(0) as usize || values.len() != *rows {
                        return Err(fail(format!("'^{}' is stale for the migrated world", name)));
                    }
                    AliasTarget::Mask { values: values.clone(), rows: *rows, schema }
                }
                AliasTarget::Resolver { resolver, carrier, service, service_declaration, input, .. } => {
                    let Some(spec) = resolver_spec(resolver) else {
                        return Err(fail(format!("'^{}' resolver '{}' is not registered", name, resolver)));
                    };
                    let contract = resolver_contract(new, spec)?;
                    if contract.carrier != *carrier || contract.service != *service
                        || contract.declaration != *service_declaration {
                        return Err(fail(format!("'^{}' resolver service or carrier changed", name)));
                    }
                    let value = (spec.run)(new, input)
                        .map_err(|_| fail(format!("'^{}' resolver is stale for the migrated world", name)))?;
                    if !value_fits(&value, *carrier, new) {
                        return Err(fail(format!("'^{}' resolver is stale for the migrated world", name)));
                    }
                    AliasTarget::Resolver {
                        resolver: resolver.clone(),
                        carrier: *carrier,
                        service: *service,
                        service_declaration: *service_declaration,
                        schema,
                        input: input.clone(),
                    }
                }
            };
            entries.insert(name.clone(), migrated);
        }
        let environment = Self {
            version: self.version.wrapping_add(1),
            schema,
            entries,
        };
        environment.snapshot(new)?;
        Ok(environment)
    }

    fn encode(&self) -> Vec<u8> {
        let mut text = String::new();
        let _ = writeln!(text, "{}\t{}\t{:016x}", MAGIC, self.version, self.schema);
        for (name, target) in &self.entries {
            match target {
                AliasTarget::Binding { binding, carrier, schema } => {
                    let _ = writeln!(
                        text,
                        "bind\t{}\t{}\t{}\t{:016x}",
                        name,
                        binding,
                        carrier.word(),
                        schema
                    );
                }
                AliasTarget::Mask { values, rows, schema } => {
                    let _ = write!(text, "mask\t{}\t{}\t{:016x}", name, rows, schema);
                    for value in values {
                        let _ = write!(text, "\t{}", num::dnum(*value));
                    }
                    text.push('\n');
                }
                // MAGIC stays ano-aliases-v1: no sidecar written before resolvers existed can
                // contain a `rslv` record, so a v1 reader that refuses the word stays sound.
                AliasTarget::Resolver { resolver, carrier, service, service_declaration, schema, input } => {
                    let service_stamp = service_stamp_word(*service_declaration, *service);
                    let _ = write!(
                        text,
                        "rslv\t{}\t{}\t{}\t{}\t{:016x}\t{}",
                        name,
                        resolver,
                        carrier.word(),
                        service_stamp,
                        schema,
                        input.version
                    );
                    for (key, value) in &input.pairs {
                        let _ = write!(text, "\t{}={}", key, value);
                    }
                    text.push('\n');
                }
            }
        }
        text.into_bytes()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Diag> {
        let path = path.as_ref();
        let encoded = self.encode();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| {
                fail(format!("cannot create '{}': {}", parent.display(), error))
            })?;
        }
        crate::fs::fs_write_commit(path, &encoded).map_err(|error| {
            fail(format!("cannot publish '{}': {}", path.display(), error))
        })?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>, reg: &Registry) -> Result<Self, Diag> {
        let path = path.as_ref();
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::for_registry(reg)),
            Err(error) => return Err(fail(format!("cannot read '{}': {}", path.display(), error))),
        };
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| fail(format!("'{}' is not a UTF-8 alias sidecar", path.display())))?;
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| fail(format!("'{}' is empty", path.display())))?;
        let fields: Vec<&str> = header.split('\t').collect();
        if fields.len() != 3 || fields[0] != MAGIC {
            return Err(fail(format!("'{}' has an unsupported header", path.display())));
        }
        let version = fields[1]
            .parse::<u64>()
            .map_err(|_| fail(format!("'{}' has a bad version", path.display())))?;
        let schema = u64::from_str_radix(fields[2], 16)
            .map_err(|_| fail(format!("'{}' has a bad schema", path.display())))?;
        let current = registry_fingerprint(reg);
        if schema != current {
            return Err(fail(format!(
                "'{}' belongs to schema {:016x}, current registry is {:016x}",
                path.display(), schema, current
            )));
        }
        let mut environment = Self { version, schema, entries: BTreeMap::new() };
        for (offset, line) in lines.enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_no = offset + 2;
            let fields: Vec<&str> = line.split('\t').collect();
            let malformed = || {
                fail(format!("{}:{}: malformed alias record", path.display(), line_no))
            };
            match fields.first().copied() {
                Some("bind") if fields.len() == 5 => {
                    validate_name(fields[1], "alias name")?;
                    let carrier = AliasCarrier::parse(fields[3]).ok_or_else(malformed)?;
                    let record_schema =
                        u64::from_str_radix(fields[4], 16).map_err(|_| malformed())?;
                    if record_schema != schema {
                        return Err(malformed());
                    }
                    let target = binding_target(reg, fields[2], schema)?;
                    if target.carrier() != carrier {
                        return Err(fail(format!(
                            "{}:{}: binding carrier changed",
                            path.display(), line_no
                        )));
                    }
                    environment.entries.insert(canonical_name(fields[1]), target);
                }
                Some("mask") if fields.len() >= 4 => {
                    validate_name(fields[1], "alias name")?;
                    let rows = fields[2].parse::<usize>().map_err(|_| malformed())?;
                    let record_schema =
                        u64::from_str_radix(fields[3], 16).map_err(|_| malformed())?;
                    if record_schema != schema || rows != reg.n.max(0) as usize {
                        return Err(malformed());
                    }
                    let mut values = Vec::with_capacity(fields.len().saturating_sub(4));
                    for spelling in &fields[4..] {
                        values.push(num::wnum(spelling).ok_or_else(malformed)?);
                    }
                    if values.len() != rows
                        || values.iter().any(|value| *value != 0.0 && *value != 1.0)
                    {
                        return Err(malformed());
                    }
                    environment.entries.insert(
                        canonical_name(fields[1]),
                        AliasTarget::Mask { values, rows, schema },
                    );
                }
                Some("rslv") if fields.len() >= 7 => {
                    validate_name(fields[1], "alias name")?;
                    let Some(spec) = resolver_spec(fields[2]) else {
                        return Err(fail(format!(
                            "{}:{}: unknown resolver '{}'",
                            path.display(), line_no, fields[2]
                        )));
                    };
                    let contract = resolver_contract(reg, spec)?;
                    let carrier = AliasCarrier::parse(fields[3]).ok_or_else(malformed)?;
                    if carrier != contract.carrier {
                        return Err(malformed());
                    }
                    let (service_declaration, service) =
                        parse_service_stamp(fields[4]).ok_or_else(malformed)?;
                    let record_schema =
                        u64::from_str_radix(fields[5], 16).map_err(|_| malformed())?;
                    if record_schema != schema {
                        return Err(malformed());
                    }
                    let version = fields[6].parse::<u64>().map_err(|_| malformed())?;
                    let mut pairs: Vec<(String, String)> = Vec::new();
                    for member in &fields[7..] {
                        let (key, value) = member.split_once('=').ok_or_else(malformed)?;
                        pairs.push((key.to_string(), value.to_string()));
                    }
                    // service drift and a stale result are caught by the final snapshot below
                    environment.entries.insert(
                        canonical_name(fields[1]),
                        AliasTarget::Resolver {
                            resolver: spec.id.to_string(),
                            carrier,
                            service,
                            service_declaration,
                            schema,
                            input: HostInput { version, pairs: input_pairs(&pairs)? },
                        },
                    );
                }
                _ => return Err(malformed()),
            }
        }
        environment.snapshot(reg)?;
        if environment.encode() != bytes {
            return Err(fail(format!(
                "'{}' is not in canonical alias form",
                path.display()
            )));
        }
        Ok(environment)
    }

    /// The operator-initiated recovery path for a sidecar this registry can no longer load, used
    /// only by `kore alias clear`.  Never automatic: a discarded environment returns at the old
    /// header's version plus one (or 1 when that field is unreadable) with the discard reason, so
    /// the version counter keeps moving forward across the reset.  The bare namespace is untouched.
    pub fn load_or_recover(path: impl AsRef<Path>, reg: &Registry) -> (Self, Option<String>) {
        let path = path.as_ref();
        match Self::load(path, reg) {
            Ok(environment) => (environment, None),
            Err(diag) => {
                let mut fresh = Self::for_registry(reg);
                fresh.version = header_version(path).unwrap_or(0).wrapping_add(1);
                (fresh, Some(diag.msg))
            }
        }
    }
}

// The version field of a sidecar header, when the file and that one field still parse.
fn header_version(path: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().next()?.split('\t').nth(1)?.parse::<u64>().ok()
}

impl AliasSnapshot {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &AliasTarget)> {
        self.entries.iter().map(|(name, target)| (name.as_str(), target))
    }

    pub fn validate(&self, reg: &Registry) -> Result<(), Diag> {
        let current = registry_fingerprint(reg);
        if self.schema != current {
            return Err(fail(format!(
                "snapshot schema {:016x} is stale for registry {:016x}",
                self.schema, current
            )));
        }
        for (name, target) in &self.entries {
            match target {
                AliasTarget::Binding { binding, carrier, schema } => {
                    if *schema != current {
                        return Err(fail(format!("'^{}' has a stale schema", name)));
                    }
                    let Some(index) = reg_find(reg, binding) else {
                        return Err(fail(format!(
                            "'^{}' targets missing binding '{}'",
                            name, binding
                        )));
                    };
                    if entry_carrier(reg, index)? != *carrier {
                        return Err(fail(format!("'^{}' binding carrier changed", name)));
                    }
                }
                AliasTarget::Mask { values, rows, schema } => {
                    if *schema != current
                        || *rows != reg.n.max(0) as usize
                        || values.len() != *rows
                        || values.iter().any(|value| *value != 0.0 && *value != 1.0)
                    {
                        return Err(fail(format!("'^{}' is a stale or malformed mask", name)));
                    }
                }
                AliasTarget::Resolver { resolver, carrier, service, service_declaration, schema, input } => {
                    if *schema != current {
                        return Err(fail(format!("'^{}' has a stale schema", name)));
                    }
                    let Some(spec) = resolver_spec(resolver) else {
                        return Err(fail(format!(
                            "'^{}' resolver '{}' is not registered",
                            name, resolver
                        )));
                    };
                    let contract = resolver_contract(reg, spec)?;
                    if contract.service != *service
                        || contract.declaration != *service_declaration {
                        return Err(fail(format!("'^{}' resolver service changed", name)));
                    }
                    let stale = || fail(format!("'^{}' resolver is stale for the current world", name));
                    let value = (spec.run)(reg, input).map_err(|_| stale())?;
                    if contract.carrier != *carrier || !value_fits(&value, *carrier, reg) {
                        return Err(stale());
                    }
                }
            }
        }
        Ok(())
    }

    /// The stored target under a stem, for host display and trace provenance.  Reading a target
    /// is not a consultation: it neither validates nor invokes.
    pub fn target(&self, name: &str) -> Option<&AliasTarget> {
        self.entries.get(&canonical_name(name))
    }

    /// `Ok(None)` means the overlay has no entry and the caller must perform ordinary bare
    /// lookup.  A present but stale entry is an error and never falls back.
    pub fn resolve(
        &self,
        reg: &Registry,
        name: &str,
    ) -> Result<Option<ResolvedAlias>, Diag> {
        let Some(target) = self.entries.get(&canonical_name(name)) else {
            return Ok(None);
        };
        match target {
            AliasTarget::Binding { binding, carrier, schema } => {
                let current = registry_fingerprint(reg);
                if *schema != current || self.schema != current {
                    return Err(fail(format!("'^{}' has a stale schema", name)));
                }
                let Some(index) = reg_find(reg, binding) else {
                    return Err(fail(format!(
                        "'^{}' targets missing binding '{}'",
                        name, binding
                    )));
                };
                if entry_carrier(reg, index)? != *carrier {
                    return Err(fail(format!("'^{}' binding carrier changed", name)));
                }
                Ok(Some(ResolvedAlias::Entry(index)))
            }
            AliasTarget::Mask { values, rows, schema } => {
                let current = registry_fingerprint(reg);
                if *schema != current
                    || self.schema != current
                    || *rows != reg.n.max(0) as usize
                    || values.len() != *rows
                {
                    return Err(fail(format!("'^{}' is stale for the current world", name)));
                }
                Ok(Some(ResolvedAlias::Mask(values.clone())))
            }
            AliasTarget::Resolver { resolver, carrier, service, service_declaration, schema, input } => {
                let current = registry_fingerprint(reg);
                if *schema != current || self.schema != current {
                    return Err(fail(format!("'^{}' has a stale schema", name)));
                }
                let Some(spec) = resolver_spec(resolver) else {
                    return Err(fail(format!(
                        "'^{}' resolver '{}' is not registered",
                        name, resolver
                    )));
                };
                let contract = resolver_contract(reg, spec)?;
                if contract.service != *service
                    || contract.declaration != *service_declaration {
                    return Err(fail(format!("'^{}' resolver service changed", name)));
                }
                let stale = || fail(format!("'^{}' resolver is stale for the current world", name));
                let value = (spec.run)(reg, input).map_err(|_| stale())?;
                if contract.carrier != *carrier || !value_fits(&value, *carrier, reg) {
                    return Err(stale());
                }
                Ok(Some(match value {
                    ResolverValue::Entity(row) => ResolvedAlias::EntityRow(row),
                    ResolverValue::Mask(values) => ResolvedAlias::Mask(values),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::names_eq;
    use crate::{RegEntry, RegEntryKind};

    fn registry() -> Registry {
        Registry {
            n: 3,
            ents: vec![
                RegEntry {
                    name: "Gold".to_string(),
                    defval: 0.0,
                    kind: RegEntryKind::Col {
                        ty: ColType::Num,
                        uniq: false,
                        nums: vec![1.0, 2.0, 3.0],
                        syms: Vec::new(),
                        pres: None,
                        rng: None,
                    },
                },
                RegEntry {
                    name: "Blast".to_string(),
                    defval: 0.0,
                    kind: RegEntryKind::Fn { body: None },
                },
            ],
            ..Registry::default()
        }
    }

    fn pairs(members: &[(&str, &str)]) -> Vec<(String, String)> {
        members.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn listing(environment: &AliasEnvironment) -> Vec<(String, AliasTarget)> {
        environment.iter().map(|(name, target)| (name.to_string(), target.clone())).collect()
    }

    // A private directory under the system temp root, unique per test and per process.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("ano-alias-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dynamic_alias_absent_falls_back_and_present_resolves() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        assert!(environment.snapshot(&reg).unwrap().resolve(&reg, "focus").unwrap().is_none());
        environment.install_binding(&reg, "focus", "gold").unwrap();
        assert_eq!(
            environment.snapshot(&reg).unwrap().resolve(&reg, "FOCUS").unwrap(),
            Some(ResolvedAlias::Entry(0))
        );
    }

    #[test]
    fn dynamic_alias_masks_pin_the_row_domain() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
        let mut changed = reg.clone();
        changed.n = 4;
        assert!(environment.snapshot(&changed).is_err());
    }

    #[test]
    fn dynamic_alias_name_fold_is_ascii_only() {
        assert_eq!(canonical_name("FoO山"), "foo山");
        assert!(names_eq(&canonical_name("FoO山"), "foo山"));
    }

    // Install, rebind the same stem, delete, delete again, clear.  Every successful transition
    // moves the version forward exactly once; the idempotent second delete moves nothing.
    #[test]
    fn dynamic_alias_lifecycle_moves_the_version_forward() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        assert_eq!(environment.version(), 0);

        environment.install_binding(&reg, "focus", "Gold").unwrap();
        assert_eq!(environment.version(), 1);
        assert_eq!(listing(&environment).len(), 1);

        environment.install_mask(&reg, "FOCUS", &[1.0, 0.0, 1.0]).unwrap();
        assert_eq!(environment.version(), 2);
        assert_eq!(listing(&environment).len(), 1);
        assert!(matches!(listing(&environment)[0].1, AliasTarget::Mask { .. }));

        assert!(environment.delete("focus"));
        assert_eq!(environment.version(), 3);
        assert!(environment.is_empty());

        assert!(!environment.delete("focus"));
        assert_eq!(environment.version(), 3);

        environment.clear(&reg);
        assert_eq!(environment.version(), 4);
    }

    // Every failing install path leaves the environment byte-for-byte where it was: the version
    // counter and the entry listing are the whole observable state of a transition.
    #[test]
    fn failed_transitions_leave_the_environment_unchanged() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment.install_binding(&reg, "keep", "Gold").unwrap();
        let version = environment.version();
        let before = listing(&environment);

        let refuse = |result: Result<(), Diag>, what: &str| {
            assert!(result.is_err(), "{} must refuse", what);
        };
        refuse(environment.install_binding(&reg, "a", "Nope"), "unregistered binding");
        refuse(environment.install_binding(&reg, "a", "Blast"), "fn target");
        refuse(environment.install_mask(&reg, "a", &[1.0, 0.0]), "mask row count");
        refuse(environment.install_mask(&reg, "a", &[1.0, 2.0, 0.0]), "mask non-0/1");
        refuse(
            environment.install_resolver(&reg, "a", "input.nope", &pairs(&[("entity", "0")])),
            "unknown resolver",
        );
        refuse(
            environment.install_resolver(&reg, "a", "input.entity", &pairs(&[("entity", "9")])),
            "out-of-domain resolver input",
        );
        refuse(
            environment.install_resolver(&reg, "a", "input.entity", &pairs(&[("ent\try", "0")])),
            "malformed input pair",
        );
        refuse(
            environment.install_resolver(&reg, "a", "input.entity", &pairs(&[("entity", "x")])),
            "unparsable resolver input",
        );

        assert_eq!(environment.version(), version);
        assert_eq!(listing(&environment), before);
    }

    // The name fold is ASCII-only, so a non-ASCII case pair is two distinct stems.
    #[test]
    fn dynamic_alias_non_ascii_case_pairs_stay_distinct() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment.install_binding(&reg, "σ", "Gold").unwrap();
        let snapshot = environment.snapshot(&reg).unwrap();
        assert!(snapshot.resolve(&reg, "Σ").unwrap().is_none());
        assert_eq!(snapshot.resolve(&reg, "σ").unwrap(), Some(ResolvedAlias::Entry(0)));
    }

    // A stem is one nonempty single-line token; the sidecar is tab-delimited and line-oriented.
    #[test]
    fn dynamic_alias_name_refusals() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        for name in ["", "a\tb", "a\nb", "a\rb"] {
            assert!(environment.install_binding(&reg, name, "Gold").is_err(), "{:?}", name);
        }
        assert_eq!(environment.version(), 0);
    }

    // A resolver target invokes once at install and again at each consultation, always against
    // the frozen (registry, input) pair recorded on the target.
    #[test]
    fn resolver_targets_install_and_resolve() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment
            .install_resolver(&reg, "focus", "input.entity", &pairs(&[("entity", "1")]))
            .unwrap();
        environment
            .install_resolver(&reg, "hot", "input.mask", &pairs(&[("mask", "1 0 1")]))
            .unwrap();
        let snapshot = environment.snapshot(&reg).unwrap();
        assert_eq!(snapshot.resolve(&reg, "FOCUS").unwrap(), Some(ResolvedAlias::EntityRow(1)));
        assert_eq!(
            snapshot.resolve(&reg, "hot").unwrap(),
            Some(ResolvedAlias::Mask(vec![1.0, 0.0, 1.0]))
        );
        // rerunning against the same frozen pair is observationally identical
        assert_eq!(snapshot.resolve(&reg, "focus").unwrap(), Some(ResolvedAlias::EntityRow(1)));
        assert_eq!(
            environment.iter().next().unwrap().1.describe(),
            "resolver input.entity (entity, service 1, input v1)"
        );
    }

    #[test]
    fn concrete_deictics_pin_their_carriers_and_inputs() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment
            .install_resolver(&reg, "cursor", "deictic.cursor", &pairs(&[("cursor", "2")]))
            .unwrap();
        environment
            .install_resolver(
                &reg,
                "observer",
                "deictic.observer",
                &pairs(&[("observer", "1")]),
            )
            .unwrap();
        environment
            .install_resolver(
                &reg,
                "selected",
                "deictic.selected",
                &pairs(&[("selected", "1 0 1")]),
            )
            .unwrap();
        environment
            .install_resolver(&reg, "world", "deictic.world", &[])
            .unwrap();

        let snapshot = environment.snapshot(&reg).unwrap();
        assert_eq!(
            snapshot.resolve(&reg, "cursor").unwrap(),
            Some(ResolvedAlias::EntityRow(2))
        );
        assert_eq!(
            snapshot.resolve(&reg, "observer").unwrap(),
            Some(ResolvedAlias::EntityRow(1))
        );
        assert_eq!(
            snapshot.resolve(&reg, "selected").unwrap(),
            Some(ResolvedAlias::Mask(vec![1.0, 0.0, 1.0]))
        );
        assert_eq!(
            snapshot.resolve(&reg, "world").unwrap(),
            Some(ResolvedAlias::Mask(vec![1.0, 1.0, 1.0]))
        );

        let before = environment.clone();
        assert!(environment
            .install_resolver(&reg, "cursor", "deictic.cursor", &pairs(&[("entity", "0")]))
            .is_err());
        assert_eq!(environment, before);
    }

    #[test]
    fn sidecar_round_trip_preserves_every_target_kind() {
        let reg = registry();
        let path = scratch("round-trip").join("world.reg.aliases");
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment.install_binding(&reg, "focus", "Gold").unwrap();
        environment.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
        environment
            .install_resolver(&reg, "cursorish", "input.entity", &pairs(&[("entity", "2")]))
            .unwrap();
        environment.save(&path).unwrap();

        let loaded = AliasEnvironment::load(&path, &reg).unwrap();
        assert_eq!(loaded, environment);
        assert_eq!(loaded.version(), environment.version());
        assert_eq!(loaded.schema(), environment.schema());
        assert_eq!(loaded.schema(), registry_fingerprint(&reg));
    }

    #[test]
    fn schema_fingerprint_ignores_world_values_and_covers_descriptors() {
        let reg = registry();
        let schema = registry_fingerprint(&reg);

        let mut world_update = reg.clone();
        let mut signed_zero = reg.clone();
        signed_zero.ents[0].defval = -0.0;
        assert_eq!(registry_fingerprint(&signed_zero), schema);

        world_update.n = 4;
        let RegEntryKind::Col { nums, .. } = &mut world_update.ents[0].kind else {
            unreachable!()
        };
        *nums = vec![9.0, 8.0, 7.0, 6.0];
        assert_eq!(registry_fingerprint(&world_update), schema);

        let mut legacy_pair = reg.clone();
        let RegEntryKind::Col { nums, .. } = &mut legacy_pair.ents[0].kind else {
            unreachable!()
        };
        *nums = vec![9.0, 8.0, 7.0, 6.0, 5.0, 4.0];
        assert_eq!(registry_fingerprint(&legacy_pair), schema);


        let mut changed = reg.clone();
        changed.ents[0].defval = 1.0;
        assert_ne!(registry_fingerprint(&changed), schema);

        let mut changed = reg.clone();
        let RegEntryKind::Col { pres, .. } = &mut changed.ents[0].kind else {
            unreachable!()
        };
        *pres = Some(vec![1.0; 3]);
        assert_ne!(registry_fingerprint(&changed), schema);

        let mut changed = reg.clone();
        let RegEntryKind::Col { rng, .. } = &mut changed.ents[0].kind else {
            unreachable!()
        };
        *rng = Some((0.0, 10.0));
        assert_ne!(registry_fingerprint(&changed), schema);

        let mut changed = reg.clone();
        let RegEntryKind::Fn { body } = &mut changed.ents[1].kind else {
            unreachable!()
        };
        *body = Some("{𝕨+𝕩}".into());
        assert_ne!(registry_fingerprint(&changed), schema);

        let mut changed = reg.clone();
        changed.reap = Some(crate::Reap::Host);
        assert_ne!(registry_fingerprint(&changed), schema);
    }

    #[test]
    fn sidecar_load_refusals() {
        let reg = registry();
        let dir = scratch("load-refusals");
        let schema = registry_fingerprint(&reg);
        let write = |name: &str, text: String| {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        };
        let header = format!("{}\t4\t{:016x}\n", MAGIC, schema);

        let bad_magic = write("magic", format!("ano-aliases-v9\t1\t{:016x}\n", schema));
        assert!(AliasEnvironment::load(&bad_magic, &reg).is_err());

        let good = write(
            "good",
            format!("{}bind\tfocus\tGold\tnumber\t{:016x}\n", header, schema),
        );

        let canonical = std::fs::read(&good).unwrap();
        let variants = vec![
            ("missing-final-newline", canonical[..canonical.len() - 1].to_vec()),
            ("blank-record", [canonical.as_slice(), b"\n"].concat()),
            ("padded-version", format!("{}\t04\t{:016x}\nbind\tfocus\tGold\tnumber\t{:016x}\n", MAGIC, schema, schema).into_bytes()),
            ("wide-schema", format!("{}\t4\t{:017x}\nbind\tfocus\tGold\tnumber\t{:016x}\n", MAGIC, schema, schema).into_bytes()),
            ("noncanonical-name", format!("{}bind\tFocus\tGold\tnumber\t{:016x}\n", header, schema).into_bytes()),
            ("duplicate-name", format!("{}bind\tfocus\tGold\tnumber\t{:016x}\nbind\tfocus\tGold\tnumber\t{:016x}\n", header, schema, schema).into_bytes()),
            ("non-utf8", { let mut bytes = canonical.clone(); bytes.push(0xff); bytes }),
        ];
        for (name, bytes) in variants {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            assert!(AliasEnvironment::load(path, &reg).is_err(), "accepted {name}");
        }

        let mut environment = AliasEnvironment::for_registry(&reg);
        let bad_input = pairs(&[("bad=key", "1")]);
        assert!(environment.install_resolver(&reg, "focus", "input.entity", &bad_input).is_err());

        let mut drifted = reg.clone();
        drifted.ents.push(RegEntry {
            name: "Silver".to_string(),
            defval: 0.0,
            kind: RegEntryKind::Col {
                ty: ColType::Num,
                uniq: false,
                nums: vec![0.0, 0.0, 0.0],
                syms: Vec::new(),
                pres: None,
                rng: None,
            },
        });
        assert!(AliasEnvironment::load(&good, &drifted).is_err());

        let malformed = write("malformed", format!("{}wat\tfocus\n", header));
        assert!(AliasEnvironment::load(&malformed, &reg).is_err());

        // n is deliberately outside the fingerprint, so a mask record pins its own row count
        let mask = write(
            "mask",
            format!("{}mask\thot\t3\t{:016x}\t1\t0\t1\n", header, schema),
        );
        let mut wider = reg.clone();
        wider.n = 4;
        assert!(AliasEnvironment::load(&mask, &wider).is_err());

        let unknown = write(
            "unknown",
            format!("{}rslv\tfocus\tinput.nope\tentity\t1\t{:016x}\t1\tentity=1\n", header, schema),
        );
        let error = AliasEnvironment::load(&unknown, &reg).unwrap_err();
        assert!(error.msg.contains("unknown resolver 'input.nope'"), "{}", error.msg);

        // the service field hand-edited under a registered resolver: caught by the final snapshot
        let drift = write(
            "service",
            format!("{}rslv\tfocus\tinput.entity\tentity\t9\t{:016x}\t1\tentity=1\n", header, schema),
        );
        let error = AliasEnvironment::load(&drift, &reg).unwrap_err();
        assert!(error.msg.contains("resolver service changed"), "{}", error.msg);
    }

    // The operator-initiated reset: a sidecar this registry cannot load is discarded with its
    // reason, and the fresh environment resumes at the old header's version plus one.
    #[test]
    fn load_or_recover_discards_a_corrupt_sidecar() {
        let reg = registry();
        let dir = scratch("recover");
        let path = dir.join("world.reg.aliases");
        std::fs::write(&path, format!("{}\t7\tnot-a-schema\n", MAGIC)).unwrap();
        let (environment, note) = AliasEnvironment::load_or_recover(&path, &reg);
        assert_eq!(environment.version(), 8);
        assert!(environment.is_empty());
        assert!(note.is_some(), "a discard must report its reason");

        let junk = dir.join("junk.aliases");
        std::fs::write(&junk, "garbage\n").unwrap();
        let (environment, note) = AliasEnvironment::load_or_recover(&junk, &reg);
        assert_eq!(environment.version(), 1);
        assert!(note.is_some());

        let absent = dir.join("absent.aliases");
        let (environment, note) = AliasEnvironment::load_or_recover(&absent, &reg);
        assert_eq!(environment.version(), 0);
        assert!(note.is_none());
    }

    // An overlay entry that is present but stale, invalid, or incompatible with its context
    // refuses at the lookup itself; bare fallback happens only when the canonical key is absent
    // from the overlay entirely.  Pinned per name against a mutated registry clone, beneath
    // AliasSnapshot::validate, which refuses the whole emission when any entry is stale.
    #[test]
    fn resolve_stale_arms_refuse_without_falling_back() {
        let reg = registry();
        let mut environment = AliasEnvironment::for_registry(&reg);
        environment.install_binding(&reg, "focus", "Gold").unwrap();
        environment.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).unwrap();
        environment
            .install_resolver(&reg, "here", "input.entity", &pairs(&[("entity", "2")]))
            .unwrap();
        let snapshot = environment.snapshot(&reg).unwrap();

        // a schema move stales the binding arm
        let mut renamed = reg.clone();
        renamed.ents[0].name = "Silver".to_string();
        let error = snapshot.resolve(&renamed, "focus").unwrap_err();
        assert!(error.msg.contains("stale schema"), "{}", error.msg);

        // n is outside the fingerprint, so the mask and resolver arms carry the row domain
        let mut wider = reg.clone();
        wider.n = 4;
        let error = snapshot.resolve(&wider, "hot").unwrap_err();
        assert!(error.msg.contains("stale for the current world"), "{}", error.msg);

        let mut narrower = reg.clone();
        narrower.n = 1;
        let error = snapshot.resolve(&narrower, "here").unwrap_err();
        assert!(error.msg.contains("resolver is stale"), "{}", error.msg);
    }
}
