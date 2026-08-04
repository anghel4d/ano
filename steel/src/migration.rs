//! Barrier-level registry migration for the pre-spatial schema.
//!
//! `.reg` remains the world/schema language. Stable declaration identities and schema versions
//! live in a host sidecar, so migration does not pre-empt the gated registry-taxonomy surface.

use crate::alias::{AliasEnvironment, canonical_name, registry_fingerprint};
use crate::registry::names_eq;
use crate::{ANO_NATMAX, BindKind, ColType, Diag, ProtoField, RegEntry, RegEntryKind, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const MANIFEST_MAGIC: &str = "ano-schema-v1";

fn fail(message: impl Into<String>) -> Diag {
    Diag::refuse(format!("migration: {}", message.into()))
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclarationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaIdentity {
    pub fingerprint: u64,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaManifest {
    pub fingerprint: u64,
    pub version: u64,
    declarations: Vec<(DeclarationId, String)>,
}

pub fn manifest_path(registry_path: &str) -> PathBuf {
    PathBuf::from(format!("{}.schema", registry_path))
}

impl SchemaManifest {
    // Input: a registry with no persisted manifest. Output: deterministic initial IDs scoped to
    // its structural fingerprint; subsequent migrations persist and carry these IDs forward.
    pub fn for_registry(registry: &Registry) -> Self {
        let fingerprint = registry_fingerprint(registry);
        let mut used = BTreeSet::new();
        let declarations = registry
            .ents
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let id = mint_id(fingerprint, 0, index, &entry.name, &mut used);
                (id, entry.name.clone())
            })
            .collect();
        Self {
            fingerprint,
            version: 0,
            declarations,
        }
    }

    pub fn identity(&self) -> SchemaIdentity {
        SchemaIdentity {
            fingerprint: self.fingerprint,
            version: self.version,
        }
    }

    pub fn declaration(&self, name: &str) -> Option<DeclarationId> {
        self.declarations
            .iter()
            .find(|(_, declared)| names_eq(declared, name))
            .map(|(id, _)| *id)
    }

    pub fn name(&self, id: DeclarationId) -> Option<&str> {
        self.declarations
            .iter()
            .find(|(declared, _)| *declared == id)
            .map(|(_, name)| name.as_str())
    }

    pub fn validate(&self, registry: &Registry) -> Result<(), Diag> {
        let fingerprint = registry_fingerprint(registry);
        if self.fingerprint != fingerprint {
            return Err(fail(format!(
                "manifest schema {:016x} is stale for registry {:016x}",
                self.fingerprint, fingerprint
            )));
        }
        if self.declarations.len() != registry.ents.len() {
            return Err(fail(
                "manifest declaration count does not match the registry",
            ));
        }
        let mut ids = BTreeSet::new();
        for ((id, name), entry) in self.declarations.iter().zip(&registry.ents) {
            if !ids.insert(*id) || !names_eq(name, &entry.name) {
                return Err(fail(
                    "manifest declaration order, name, or identity is invalid",
                ));
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut text = format!(
            "{}\t{}\t{:016x}\n",
            MANIFEST_MAGIC, self.version, self.fingerprint
        );
        for (id, name) in &self.declarations {
            let _ = writeln!(text, "decl\t{:016x}\t{}", id.0, name);
        }
        text.into_bytes()
    }

    pub fn load(path: impl AsRef<Path>, registry: &Registry) -> Result<Self, Diag> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::for_registry(registry));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|error| fail(format!("cannot read '{}': {}", path.display(), error)))?;
        let mut lines = text.lines();
        let fields: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
        if fields.len() != 3 || fields[0] != MANIFEST_MAGIC {
            return Err(fail(format!(
                "'{}' has an unsupported schema manifest",
                path.display()
            )));
        }
        let version = fields[1]
            .parse::<u64>()
            .map_err(|_| fail("bad schema version"))?;
        let fingerprint =
            u64::from_str_radix(fields[2], 16).map_err(|_| fail("bad schema fingerprint"))?;
        let mut declarations = Vec::new();
        for (offset, line) in lines.enumerate() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 3 || fields[0] != "decl" {
                return Err(fail(format!(
                    "{}:{}: malformed declaration identity",
                    path.display(),
                    offset + 2
                )));
            }
            let id = u64::from_str_radix(fields[1], 16).map_err(|_| {
                fail(format!(
                    "{}:{}: malformed declaration identity",
                    path.display(),
                    offset + 2
                ))
            })?;
            declarations.push((DeclarationId(id), fields[2].to_string()));
        }
        let manifest = Self {
            fingerprint,
            version,
            declarations,
        };
        manifest.validate(registry)?;
        Ok(manifest)
    }
}

fn mint_id(
    fingerprint: u64,
    version: u64,
    index: usize,
    name: &str,
    used: &mut BTreeSet<DeclarationId>,
) -> DeclarationId {
    let mut nonce = 0u64;
    loop {
        let mut hash = hash_bytes(0xcbf29ce484222325, &fingerprint.to_le_bytes());
        hash = hash_bytes(hash, &version.to_le_bytes());
        hash = hash_bytes(hash, &(index as u64).to_le_bytes());
        hash = hash_bytes(hash, &nonce.to_le_bytes());
        hash = hash_bytes(hash, name.as_bytes());
        let id = DeclarationId(if hash == 0 { 1 } else { hash });
        if used.insert(id) {
            return id;
        }
        nonce = nonce.wrapping_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    Preserve,
    Widen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    Map {
        old: String,
        new: String,
        conversion: Conversion,
    },
    Remove {
        old: String,
        discard_live: bool,
    },
    Add {
        new: String,
    },
    Unalias {
        name: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationPlan {
    rules: Vec<Rule>,
}

fn map_rule_text(line: &str) -> &str {
    let comment = line.char_indices().find_map(|(offset, character)| {
        (character == '#'
            && (offset == 0
                || line[..offset]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)))
        .then_some(offset)
    });
    comment.map_or(line, |offset| &line[..offset])
}

impl MigrationPlan {
    // Host migration-map language, deliberately separate from `.reg` and `.ano`:
    // preserve OLD [NEW], rename OLD NEW, widen OLD NEW, drop OLD, discard OLD, add NEW,
    // unalias NAME. A word-boundary # begins a comment.
    pub fn parse(text: &str) -> Result<Self, Diag> {
        let mut rules = Vec::new();
        for (offset, physical) in text.lines().enumerate() {
            let line = map_rule_text(physical).trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let malformed = || {
                fail(format!(
                    "map line {}: expected a migration rule",
                    offset + 1
                ))
            };
            let rule = match words.as_slice() {
                ["preserve", old] => Rule::Map {
                    old: (*old).into(),
                    new: (*old).into(),
                    conversion: Conversion::Preserve,
                },
                ["preserve", old, new] | ["rename", old, new] => Rule::Map {
                    old: (*old).into(),
                    new: (*new).into(),
                    conversion: Conversion::Preserve,
                },
                ["widen", old, new] => Rule::Map {
                    old: (*old).into(),
                    new: (*new).into(),
                    conversion: Conversion::Widen,
                },
                ["drop", old] => Rule::Remove {
                    old: (*old).into(),
                    discard_live: false,
                },
                ["discard", old] => Rule::Remove {
                    old: (*old).into(),
                    discard_live: true,
                },
                ["add", new] => Rule::Add { new: (*new).into() },
                ["unalias", name] => Rule::Unalias {
                    name: name.trim_start_matches('^').into(),
                },
                _ => return Err(malformed()),
            };
            rules.push(rule);
        }
        Ok(Self { rules })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanStamp(pub SchemaIdentity);

impl PlanStamp {
    pub fn valid_for(self, manifest: &SchemaManifest) -> bool {
        self.0 == manifest.identity()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclarationHandle {
    pub schema: SchemaIdentity,
    pub declaration: DeclarationId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationMigration {
    pub id: DeclarationId,
    pub old_name: String,
    pub new_name: String,
    pub conversion: Conversion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheInvalidation {
    pub compiled_plans: bool,
    pub callables: bool,
    pub views: bool,
    pub services: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReceipt {
    pub old_schema: SchemaIdentity,
    pub new_schema: SchemaIdentity,
    pub event: u64,
    pub declarations: Vec<DeclarationMigration>,
    pub removed: Vec<DeclarationId>,
    pub added: Vec<DeclarationId>,
    pub invalidation: CacheInvalidation,
}

impl MigrationReceipt {
    pub fn revalidate(&self, handle: DeclarationHandle) -> Result<DeclarationHandle, Diag> {
        if handle.schema != self.old_schema {
            return Err(fail(
                "declaration handle belongs to a different schema generation",
            ));
        }
        if self.removed.contains(&handle.declaration) {
            return Err(fail("declaration handle was removed by the migration"));
        }
        if self
            .declarations
            .iter()
            .any(|mapping| mapping.id == handle.declaration)
        {
            return Ok(DeclarationHandle {
                schema: self.new_schema,
                declaration: handle.declaration,
            });
        }
        Err(fail("declaration handle is stale or incompatible"))
    }

    pub fn log_line(&self) -> String {
        format!(
            "ano-migration-v1\t{}:{:016x}\t{}:{:016x}\t{:016x}\n",
            self.old_schema.version,
            self.old_schema.fingerprint,
            self.new_schema.version,
            self.new_schema.fingerprint,
            self.event
        )
    }
}

#[derive(Debug, Clone)]
pub struct MigrationOutcome {
    pub registry: Registry,
    pub aliases: AliasEnvironment,
    pub manifest: SchemaManifest,
    pub receipt: MigrationReceipt,
}

pub trait MigrationExtension {
    // `99` implements this seam for a changed lattice/header and its spatial descriptors. Returning
    // false leaves the pre-spatial refusal in force; extensions may not bypass final validation.
    fn migrate_header(&self, _old: &Registry, _candidate: &mut Registry) -> Result<bool, Diag> {
        Ok(false)
    }
}

pub struct NoSpatialMigration;
impl MigrationExtension for NoSpatialMigration {}

pub fn migrate(
    old: &Registry,
    candidate: &Registry,
    aliases: &AliasEnvironment,
    manifest: &SchemaManifest,
    plan: &MigrationPlan,
) -> Result<MigrationOutcome, Diag> {
    migrate_with_extension(old, candidate, aliases, manifest, plan, &NoSpatialMigration)
}

pub fn migrate_with_extension(
    old: &Registry,
    candidate: &Registry,
    aliases: &AliasEnvironment,
    manifest: &SchemaManifest,
    plan: &MigrationPlan,
    extension: &dyn MigrationExtension,
) -> Result<MigrationOutcome, Diag> {
    manifest.validate(old)?;
    aliases.snapshot(old)?;
    if old.n != candidate.n {
        return Err(fail("schema replacement cannot change the live population"));
    }
    let mut next = candidate.clone();
    if (old.lat_w, old.lat_h) != (candidate.lat_w, candidate.lat_h)
        && !extension.migrate_header(old, &mut next)?
    {
        return Err(fail(
            "lattice migration belongs to the spatial extension boundary",
        ));
    }

    let mut old_seen = vec![false; old.ents.len()];
    let mut new_seen = vec![false; candidate.ents.len()];
    let mut mappings = Vec::new();
    let mut removals = Vec::new();
    let mut additions = Vec::new();
    let mut dropped_aliases = BTreeSet::new();
    for rule in &plan.rules {
        match rule {
            Rule::Map {
                old: old_name,
                new: new_name,
                conversion,
            } => {
                let oi = declaration_index(old, old_name)
                    .ok_or_else(|| fail(format!("no old declaration '{}'", old_name)))?;
                let ni = declaration_index(candidate, new_name)
                    .ok_or_else(|| fail(format!("no candidate declaration '{}'", new_name)))?;
                mark_once(&mut old_seen, oi, "old", old_name)?;
                mark_once(&mut new_seen, ni, "candidate", new_name)?;
                mappings.push((oi, ni, *conversion));
            }
            Rule::Remove {
                old: old_name,
                discard_live,
            } => {
                let oi = declaration_index(old, old_name)
                    .ok_or_else(|| fail(format!("no old declaration '{}'", old_name)))?;
                mark_once(&mut old_seen, oi, "old", old_name)?;
                if has_live_data(old, &old.ents[oi]) && !discard_live {
                    return Err(fail(format!(
                        "declaration '{}' has live data; use an explicit discard",
                        old.ents[oi].name
                    )));
                }
                removals.push(oi);
            }
            Rule::Add { new: new_name } => {
                let ni = declaration_index(candidate, new_name)
                    .ok_or_else(|| fail(format!("no candidate declaration '{}'", new_name)))?;
                mark_once(&mut new_seen, ni, "candidate", new_name)?;
                additions.push(ni);
            }
            Rule::Unalias { name } => {
                let folded = canonical_name(name);
                if !dropped_aliases.insert(folded.clone()) {
                    return Err(fail(format!("dynamic alias '^{}' is handled twice", name)));
                }
                if aliases.iter().all(|(existing, _)| existing != folded) {
                    return Err(fail(format!("dynamic alias '^{}' does not exist", name)));
                }
            }
        }
    }
    require_complete(old, candidate, &old_seen, &new_seen)?;
    let old_to_new: BTreeMap<usize, usize> =
        mappings.iter().map(|(old, new, _)| (*old, *new)).collect();
    for &(oi, ni, conversion) in &mappings {
        validate_structure(old, candidate, oi, ni, conversion, &old_to_new)?;
        copy_world_data(&old.ents[oi], &mut next.ents[ni]);
    }
    rebuild_inverses(&mut next)?;
    for index in 0..next.ents.len() {
        validate_values(&next, index)?;
    }
    crate::relationship::validate_registry(&next)?;

    let binding_map = mappings
        .iter()
        .map(|(oi, ni, _)| {
            (
                canonical_name(&old.ents[*oi].name),
                next.ents[*ni].name.clone(),
            )
        })
        .collect();
    let next_aliases = aliases.migrate_schema(old, &next, &binding_map, &dropped_aliases)?;
    let next_fingerprint = registry_fingerprint(&next);
    let mut used: BTreeSet<DeclarationId> =
        manifest.declarations.iter().map(|(id, _)| *id).collect();
    let mut declarations = vec![(DeclarationId(0), String::new()); next.ents.len()];
    let mut receipt_mappings = Vec::new();
    for &(oi, ni, conversion) in &mappings {
        let id = manifest.declarations[oi].0;
        declarations[ni] = (id, next.ents[ni].name.clone());
        receipt_mappings.push(DeclarationMigration {
            id,
            old_name: old.ents[oi].name.clone(),
            new_name: next.ents[ni].name.clone(),
            conversion,
        });
    }
    let next_version = manifest
        .version
        .checked_add(1)
        .ok_or_else(|| fail("schema version is exhausted"))?;
    let mut added_ids = Vec::new();
    for ni in additions {
        let id = mint_id(
            next_fingerprint,
            next_version,
            ni,
            &next.ents[ni].name,
            &mut used,
        );
        declarations[ni] = (id, next.ents[ni].name.clone());
        added_ids.push(id);
    }
    if declarations.iter().any(|(id, _)| id.0 == 0) {
        return Err(fail(
            "internal migration map left a declaration without an identity",
        ));
    }
    let next_manifest = SchemaManifest {
        fingerprint: next_fingerprint,
        version: next_version,
        declarations,
    };
    next_manifest.validate(&next)?;
    let old_schema = manifest.identity();
    let new_schema = next_manifest.identity();
    let removed = removals
        .iter()
        .map(|&index| manifest.declarations[index].0)
        .collect::<Vec<_>>();
    let event = event_hash(old_schema, new_schema, plan, &next, &next_aliases);
    let receipt = MigrationReceipt {
        old_schema,
        new_schema,
        event,
        declarations: receipt_mappings,
        removed,
        added: added_ids,
        invalidation: CacheInvalidation {
            compiled_plans: true,
            callables: true,
            views: true,
            services: true,
        },
    };
    Ok(MigrationOutcome {
        registry: next,
        aliases: next_aliases,
        manifest: next_manifest,
        receipt,
    })
}

fn declaration_index(registry: &Registry, name: &str) -> Option<usize> {
    registry
        .ents
        .iter()
        .position(|entry| names_eq(&entry.name, name))
}

fn mark_once(seen: &mut [bool], index: usize, side: &str, name: &str) -> Result<(), Diag> {
    if std::mem::replace(&mut seen[index], true) {
        return Err(fail(format!(
            "{} declaration '{}' is handled twice",
            side, name
        )));
    }
    Ok(())
}

fn require_complete(
    old: &Registry,
    new: &Registry,
    old_seen: &[bool],
    new_seen: &[bool],
) -> Result<(), Diag> {
    if let Some((index, _)) = old_seen.iter().enumerate().find(|(_, seen)| !**seen) {
        return Err(fail(format!(
            "old declaration '{}' has no migration rule",
            old.ents[index].name
        )));
    }
    if let Some((index, _)) = new_seen.iter().enumerate().find(|(_, seen)| !**seen) {
        return Err(fail(format!(
            "candidate declaration '{}' has no migration rule",
            new.ents[index].name
        )));
    }
    Ok(())
}

fn is_vector(entry: &RegEntry, rows: i32) -> bool {
    matches!(&entry.kind, RegEntryKind::Col { ty: ColType::Num, nums, .. } | RegEntryKind::Field { ty: ColType::Num, nums, .. } if rows > 0 && nums.len() as i64 == 2 * rows as i64)
}

fn endpoint_maps(
    old: &Registry,
    new: &Registry,
    old_name: &Option<String>,
    new_name: &Option<String>,
    map: &BTreeMap<usize, usize>,
) -> bool {
    match (old_name, new_name) {
        (None, None) => true,
        (Some(old_name), Some(new_name)) => {
            let Some(oi) = declaration_index(old, old_name) else {
                return false;
            };
            let Some(ni) = declaration_index(new, new_name) else {
                return false;
            };
            map.get(&oi) == Some(&ni)
        }
        _ => false,
    }
}

fn reference_maps(
    old: &Registry,
    new: &Registry,
    old_name: &str,
    new_name: &str,
    map: &BTreeMap<usize, usize>,
) -> bool {
    let Some(oi) = declaration_index(old, old_name) else {
        return false;
    };
    let Some(ni) = declaration_index(new, new_name) else {
        return false;
    };
    map.get(&oi) == Some(&ni)
}

fn widening(old: ColType, new: ColType) -> bool {
    matches!(
        (old, new),
        (ColType::Bool, ColType::Nat | ColType::Int | ColType::Num)
            | (ColType::Nat, ColType::Int | ColType::Num)
            | (ColType::Int, ColType::Num)
    )
}

fn validate_structure(
    old: &Registry,
    new: &Registry,
    oi: usize,
    ni: usize,
    conversion: Conversion,
    map: &BTreeMap<usize, usize>,
) -> Result<(), Diag> {
    let (old_entry, new_entry) = (&old.ents[oi], &new.ents[ni]);
    let incompatible = || {
        fail(format!(
            "'{}' is incompatible with '{}'",
            old_entry.name, new_entry.name
        ))
    };
    match (&old_entry.kind, &new_entry.kind) {
        (
            RegEntryKind::Col {
                ty: old_ty,
                uniq: old_unique,
                pres: old_presence,
                ..
            },
            RegEntryKind::Col {
                ty: new_ty,
                uniq: new_unique,
                pres: new_presence,
                ..
            },
        ) => {
            let old_vector = is_vector(old_entry, old.n);
            let new_vector = is_vector(new_entry, new.n);
            if old_unique != new_unique
                || old_presence.is_some() != new_presence.is_some()
                || old_vector != new_vector
            {
                return Err(incompatible());
            }
            let carriers = match conversion {
                Conversion::Preserve => old_ty == new_ty,
                Conversion::Widen => !old_vector && widening(*old_ty, *new_ty),
            };
            if !carriers {
                return Err(incompatible());
            }
        }
        (RegEntryKind::Field { ty: old_ty, .. }, RegEntryKind::Field { ty: new_ty, .. }) => {
            let old_vector = is_vector(old_entry, old.lat_w.wrapping_mul(old.lat_h));
            let new_vector = is_vector(new_entry, new.lat_w.wrapping_mul(new.lat_h));
            let carriers = match conversion {
                Conversion::Preserve => old_ty == new_ty && old_vector == new_vector,
                Conversion::Widen => !old_vector && !new_vector && widening(*old_ty, *new_ty),
            };
            if !carriers {
                return Err(incompatible());
            }
        }
        (
            RegEntryKind::Rel {
                key_of: old_key, ..
            },
            RegEntryKind::Rel {
                key_of: new_key, ..
            },
        ) => {
            if conversion != Conversion::Preserve || !endpoint_maps(old, new, old_key, new_key, map)
            {
                return Err(fail(format!(
                    "relationship endpoint changed for '{}'",
                    old_entry.name
                )));
            }
        }
        (
            RegEntryKind::SRel {
                inv_of: old_inverse,
                key_of: old_key,
                ..
            },
            RegEntryKind::SRel {
                inv_of: new_inverse,
                key_of: new_key,
                ..
            },
        ) => {
            let inverse = match (old_inverse, new_inverse) {
                (None, None) => true,
                (Some(old_rel), Some(new_rel)) => reference_maps(old, new, old_rel, new_rel, map),
                _ => false,
            };
            if conversion != Conversion::Preserve
                || !inverse
                || !endpoint_maps(old, new, old_key, new_key, map)
            {
                return Err(fail(format!(
                    "set relationship endpoint changed for '{}'",
                    old_entry.name
                )));
            }
        }
        (RegEntryKind::AliasMask { .. }, RegEntryKind::AliasMask { .. }) => {}
        (RegEntryKind::Bind { kind: old_kind, .. }, RegEntryKind::Bind { kind: new_kind, .. })
            if old_kind == new_kind => {}
        (RegEntryKind::Fn { body: old_body }, RegEntryKind::Fn { body: new_body })
            if old_body == new_body => {}
        (
            RegEntryKind::Tag {
                col: old_col,
                carrier_ty: old_ty,
                num: old_num,
                sym: old_sym,
            },
            RegEntryKind::Tag {
                col: new_col,
                carrier_ty: new_ty,
                num: new_num,
                sym: new_sym,
            },
        ) if old_ty == new_ty
            && old_num == new_num
            && old_sym == new_sym
            && reference_maps(old, new, old_col, new_col, map) => {}
        (
            RegEntryKind::Proto { fields: old_fields },
            RegEntryKind::Proto { fields: new_fields },
        ) if proto_maps(old, new, old_fields, new_fields, map) => {}
        _ => return Err(incompatible()),
    }
    if conversion == Conversion::Widen
        && !matches!(
            (&old_entry.kind, &new_entry.kind),
            (RegEntryKind::Col { .. }, RegEntryKind::Col { .. })
                | (RegEntryKind::Field { .. }, RegEntryKind::Field { .. })
        )
    {
        return Err(incompatible());
    }
    Ok(())
}

fn proto_maps(
    old: &Registry,
    new: &Registry,
    old_fields: &[ProtoField],
    new_fields: &[ProtoField],
    map: &BTreeMap<usize, usize>,
) -> bool {
    old_fields.len() == new_fields.len()
        && old_fields
            .iter()
            .zip(new_fields)
            .all(|(old_field, new_field)| {
                old_field.spelling == new_field.spelling
                    && old_field.num == new_field.num
                    && reference_maps(old, new, &old_field.col, &new_field.col, map)
            })
}

fn copy_world_data(old: &RegEntry, new: &mut RegEntry) {
    match (&old.kind, &mut new.kind) {
        (
            RegEntryKind::Col {
                nums, syms, pres, ..
            },
            RegEntryKind::Col {
                nums: new_nums,
                syms: new_syms,
                pres: new_pres,
                ..
            },
        ) => {
            *new_nums = nums.clone();
            *new_syms = syms.clone();
            *new_pres = pres.clone();
        }
        (
            RegEntryKind::Field { nums, syms, .. },
            RegEntryKind::Field {
                nums: new_nums,
                syms: new_syms,
                ..
            },
        ) => {
            *new_nums = nums.clone();
            *new_syms = syms.clone();
        }
        (
            RegEntryKind::Rel { targets, .. },
            RegEntryKind::Rel {
                targets: new_targets,
                ..
            },
        ) => *new_targets = targets.clone(),
        (
            RegEntryKind::SRel {
                fib, inv_of: None, ..
            },
            RegEntryKind::SRel {
                fib: new_fib,
                inv_of: None,
                ..
            },
        ) => *new_fib = fib.clone(),
        (RegEntryKind::AliasMask { mask }, RegEntryKind::AliasMask { mask: new_mask }) => {
            *new_mask = mask.clone()
        }
        (RegEntryKind::Bind { vals, .. }, RegEntryKind::Bind { vals: new_vals, .. }) => {
            *new_vals = vals.clone()
        }
        _ => {}
    }
}

fn carrier_admits(carrier: ColType, value: f64) -> bool {
    match carrier {
        ColType::Num => !value.is_nan(),
        ColType::Bool => value == 0.0 || value == 1.0,
        ColType::Nat => value >= 0.0 && value <= ANO_NATMAX && value.fract() == 0.0,
        ColType::Int => value >= -ANO_NATMAX && value <= ANO_NATMAX && value.fract() == 0.0,
        ColType::Sym | ColType::Char => value.is_finite(),
    }
}

fn validate_values(registry: &Registry, index: usize) -> Result<(), Diag> {
    let entry = &registry.ents[index];
    let check_column =
        |carrier: ColType, values: &[f64], range: Option<(f64, f64)>| -> Result<(), Diag> {
            for value in values {
                if !carrier_admits(carrier, *value)
                    || range.is_some_and(|(lo, hi)| *value < lo || *value > hi)
                {
                    return Err(fail(format!(
                        "converted value in '{}' is outside the candidate carrier",
                        entry.name
                    )));
                }
            }
            if !carrier_admits(carrier, entry.defval)
                || range.is_some_and(|(lo, hi)| entry.defval < lo || entry.defval > hi)
            {
                return Err(fail(format!(
                    "candidate default for '{}' is outside its carrier",
                    entry.name
                )));
            }
            Ok(())
        };
    match &entry.kind {
        RegEntryKind::Col {
            ty,
            uniq,
            nums,
            pres,
            rng,
            ..
        } => {
            check_column(*ty, nums, *rng)?;
            if let Some(mask) = pres
                && (mask.len() != registry.n.max(0) as usize
                    || mask.iter().any(|value| *value != 0.0 && *value != 1.0))
            {
                return Err(fail(format!(
                    "component presence for '{}' is malformed",
                    entry.name
                )));
            }
            if *uniq {
                for left in 0..nums.len() {
                    if nums[left + 1..].contains(&nums[left]) {
                        return Err(fail(format!(
                            "converted unique column '{}' contains a duplicate",
                            entry.name
                        )));
                    }
                }
            }
        }
        RegEntryKind::Field { ty, nums, rng, .. } => check_column(*ty, nums, *rng)?,
        RegEntryKind::AliasMask { mask }
            if mask.len() != registry.n.max(0) as usize
                || mask.iter().any(|value| *value != 0.0 && *value != 1.0) =>
        {
            return Err(fail(format!(
                "static alias mask '{}' is malformed",
                entry.name
            )));
        }
        RegEntryKind::Bind {
            kind: BindKind::Mask,
            vals,
        } if vals.len() != registry.n.max(0) as usize
            || vals.iter().any(|value| *value != 0.0 && *value != 1.0) =>
        {
            return Err(fail(format!("mask binding '{}' is malformed", entry.name)));
        }
        _ => {}
    }
    Ok(())
}

fn has_live_data(registry: &Registry, entry: &RegEntry) -> bool {
    match &entry.kind {
        RegEntryKind::Col { pres: None, .. } => registry.n > 0,
        RegEntryKind::Col {
            pres: Some(mask), ..
        } => mask.iter().any(|value| *value != 0.0),
        RegEntryKind::Field { .. } => registry.lat_w > 0 && registry.lat_h > 0,
        RegEntryKind::Rel { targets, .. } => targets.iter().any(|target| *target != -1.0),
        RegEntryKind::SRel {
            fib, inv_of: None, ..
        } => fib.iter().any(|fiber| !fiber.is_empty()),
        RegEntryKind::AliasMask { mask } => mask.iter().any(|value| *value != 0.0),
        RegEntryKind::Bind { vals, .. } => !vals.is_empty(),
        RegEntryKind::SRel {
            inv_of: Some(_), ..
        }
        | RegEntryKind::Fn { .. }
        | RegEntryKind::Tag { .. }
        | RegEntryKind::Proto { .. } => false,
    }
}

fn rebuild_inverses(registry: &mut Registry) -> Result<(), Diag> {
    let mut replacements = Vec::new();
    for (index, entry) in registry.ents.iter().enumerate() {
        let RegEntryKind::SRel {
            inv_of: Some(rel_name),
            ..
        } = &entry.kind
        else {
            continue;
        };
        let rel_index = declaration_index(registry, rel_name).ok_or_else(|| {
            fail(format!(
                "inverse '{}' lost relationship '{}'",
                entry.name, rel_name
            ))
        })?;
        let (targets, key_of) = match &registry.ents[rel_index].kind {
            RegEntryKind::Rel { targets, key_of } => (targets, key_of),
            _ => {
                return Err(fail(format!(
                    "inverse '{}' target is not a relationship",
                    entry.name
                )));
            }
        };
        let keys = key_of
            .as_ref()
            .map(|name| {
                let key_index = declaration_index(registry, name)
                    .ok_or_else(|| fail(format!("missing key column '{}'", name)))?;
                match &registry.ents[key_index].kind {
                    RegEntryKind::Col { nums, .. } => Ok(nums.clone()),
                    _ => Err(fail(format!("relationship key '{}' is not a column", name))),
                }
            })
            .transpose()?;
        let mut fibers = Vec::with_capacity(registry.n.max(0) as usize);
        for target_row in 0..registry.n.max(0) as usize {
            let target_key = keys
                .as_ref()
                .map_or(target_row as f64, |values| values[target_row]);
            let mut fiber = Vec::new();
            for (source_row, target) in targets.iter().enumerate() {
                if *target == target_key {
                    fiber.push(
                        keys.as_ref()
                            .map_or(source_row as f64, |values| values[source_row]),
                    );
                }
            }
            fibers.push(fiber);
        }
        replacements.push((index, fibers));
    }
    for (index, fibers) in replacements {
        let RegEntryKind::SRel { fib, .. } = &mut registry.ents[index].kind else {
            unreachable!()
        };
        *fib = fibers;
    }
    Ok(())
}

fn event_hash(
    old: SchemaIdentity,
    new: SchemaIdentity,
    plan: &MigrationPlan,
    registry: &Registry,
    aliases: &AliasEnvironment,
) -> u64 {
    let mut hash = hash_bytes(0xcbf29ce484222325, &old.fingerprint.to_le_bytes());
    hash = hash_bytes(hash, &old.version.to_le_bytes());
    hash = hash_bytes(hash, &new.fingerprint.to_le_bytes());
    hash = hash_bytes(hash, &new.version.to_le_bytes());
    hash = hash_bytes(hash, format!("{:?}", plan).as_bytes());
    hash = hash_bytes(hash, format!("{:?}", registry).as_bytes());
    hash_bytes(hash, format!("{:?}", aliases).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegEntry;

    fn col(name: &str, ty: ColType, values: &[f64]) -> RegEntry {
        RegEntry {
            name: name.into(),
            defval: 0.0,
            kind: RegEntryKind::Col {
                ty,
                uniq: false,
                nums: values.to_vec(),
                syms: Vec::new(),
                pres: None,
                rng: None,
            },
        }
    }

    fn unique(name: &str, values: &[f64]) -> RegEntry {
        let mut entry = col(name, ColType::Nat, values);
        let RegEntryKind::Col { uniq, .. } = &mut entry.kind else {
            unreachable!()
        };
        *uniq = true;
        entry
    }

    fn worlds() -> (Registry, Registry) {
        let old = Registry {
            n: 2,
            ents: vec![
                unique("Id", &[10.0, 20.0]),
                col("Flag", ColType::Nat, &[1.0, 0.0]),
                RegEntry {
                    name: "Parent".into(),
                    defval: 0.0,
                    kind: RegEntryKind::Rel {
                        targets: vec![20.0, -1.0],
                        key_of: Some("Id".into()),
                    },
                },
                RegEntry {
                    name: "Blend".into(),
                    defval: 0.0,
                    kind: RegEntryKind::Fn {
                        body: Some("{𝕨+𝕩}".into()),
                    },
                },
            ],
            ..Registry::default()
        };
        let candidate = Registry {
            n: 2,
            ents: vec![
                unique("Id", &[0.0, 1.0]),
                col("Enabled", ColType::Int, &[0.0, 0.0]),
                RegEntry {
                    name: "Parent".into(),
                    defval: 0.0,
                    kind: RegEntryKind::Rel {
                        targets: vec![-1.0, -1.0],
                        key_of: Some("Id".into()),
                    },
                },
                RegEntry {
                    name: "Blend".into(),
                    defval: 0.0,
                    kind: RegEntryKind::Fn {
                        body: Some("{𝕨+𝕩}".into()),
                    },
                },
                col("Added", ColType::Num, &[7.0, 8.0]),
            ],
            ..Registry::default()
        };
        (old, candidate)
    }

    fn plan() -> MigrationPlan {
        MigrationPlan::parse(
            "preserve Id\nwiden Flag Enabled\npreserve Parent\npreserve Blend\nadd Added\n",
        )
        .unwrap()
    }

    #[test]
    fn rename_widen_ids_aliases_and_handles_move_together() {
        let (old, candidate) = worlds();
        let manifest = SchemaManifest::for_registry(&old);
        let flag_id = manifest.declaration("Flag").unwrap();
        let handle = DeclarationHandle {
            schema: manifest.identity(),
            declaration: flag_id,
        };
        let blend_id = manifest.declaration("Blend").unwrap();
        let callable = DeclarationHandle {
            schema: manifest.identity(),
            declaration: blend_id,
        };
        let mut aliases = AliasEnvironment::for_registry(&old);
        aliases.install_binding(&old, "focus", "Flag").unwrap();
        let outcome = migrate(&old, &candidate, &aliases, &manifest, &plan()).unwrap();
        assert_eq!(outcome.manifest.declaration("Enabled"), Some(flag_id));
        assert_eq!(
            outcome.receipt.revalidate(handle).unwrap().schema,
            outcome.manifest.identity()
        );
        let callable = outcome.receipt.revalidate(callable).unwrap();
        assert_eq!(callable.declaration, blend_id);
        assert_eq!(callable.schema, outcome.manifest.identity());
        assert!(
            outcome
                .receipt
                .revalidate(callable)
                .unwrap_err()
                .msg
                .contains("different schema generation")
        );
        assert!(!PlanStamp(manifest.identity()).valid_for(&outcome.manifest));
        let target = outcome
            .aliases
            .snapshot(&outcome.registry)
            .unwrap()
            .resolve(&outcome.registry, "focus")
            .unwrap();
        assert_eq!(target, Some(crate::alias::ResolvedAlias::Entry(1)));
        let RegEntryKind::Col { nums, .. } = &outcome.registry.ents[1].kind else {
            unreachable!()
        };
        assert_eq!(nums, &[1.0, 0.0]);
        assert!(outcome.receipt.invalidation.compiled_plans);
        assert!(outcome.receipt.invalidation.callables);
        assert!(outcome.receipt.invalidation.views);
        assert!(outcome.receipt.invalidation.services);
    }

    #[test]
    fn widening_must_be_explicit_and_data_must_fit() {
        let (old, candidate) = worlds();
        let aliases = AliasEnvironment::for_registry(&old);
        let manifest = SchemaManifest::for_registry(&old);
        let implicit = MigrationPlan::parse(
            "preserve Id\nrename Flag Enabled\npreserve Parent\npreserve Blend\nadd Added\n",
        )
        .unwrap();
        assert!(migrate(&old, &candidate, &aliases, &manifest, &implicit).is_err());
        let mut narrowed = candidate.clone();
        let RegEntryKind::Col { rng, .. } = &mut narrowed.ents[1].kind else {
            unreachable!()
        };
        *rng = Some((0.0, 0.0));
        assert!(migrate(&old, &narrowed, &aliases, &manifest, &plan()).is_err());
    }

    #[test]
    fn live_removal_requires_discard_and_removed_aliases_are_explicit() {
        let (old, mut candidate) = worlds();
        candidate.ents.remove(1);
        candidate.ents.pop();
        let manifest = SchemaManifest::for_registry(&old);
        let aliases = AliasEnvironment::for_registry(&old);
        let refused =
            MigrationPlan::parse("preserve Id\ndrop Flag\npreserve Parent\npreserve Blend\n")
                .unwrap();
        assert!(
            migrate(&old, &candidate, &aliases, &manifest, &refused)
                .unwrap_err()
                .msg
                .contains("live data")
        );
        let accepted =
            MigrationPlan::parse("preserve Id\ndiscard Flag\npreserve Parent\npreserve Blend\n")
                .unwrap();
        assert!(migrate(&old, &candidate, &aliases, &manifest, &accepted).is_ok());

        let mut aliases = AliasEnvironment::for_registry(&old);
        aliases.install_binding(&old, "focus", "Flag").unwrap();
        assert!(migrate(&old, &candidate, &aliases, &manifest, &accepted).is_err());
        let unaliased = MigrationPlan::parse(
            "preserve Id\ndiscard Flag\npreserve Parent\npreserve Blend\nunalias ^focus\n",
        )
        .unwrap();
        assert!(
            migrate(&old, &candidate, &aliases, &manifest, &unaliased)
                .unwrap()
                .aliases
                .is_empty()
        );
    }

    #[test]
    fn endpoint_and_callable_changes_refuse() {
        let (old, mut candidate) = worlds();
        candidate.ents.insert(1, unique("Other", &[1.0, 2.0]));
        let RegEntryKind::Rel { key_of, .. } = &mut candidate.ents[3].kind else {
            unreachable!()
        };
        *key_of = Some("Other".into());
        let endpoint_plan = MigrationPlan::parse(
            "preserve Id\nadd Other\nwiden Flag Enabled\npreserve Parent\npreserve Blend\nadd Added\n",
        ).unwrap();
        let manifest = SchemaManifest::for_registry(&old);
        let aliases = AliasEnvironment::for_registry(&old);
        assert!(
            migrate(&old, &candidate, &aliases, &manifest, &endpoint_plan)
                .unwrap_err()
                .msg
                .contains("endpoint")
        );

        let (_, mut changed_fn) = worlds();
        let RegEntryKind::Fn { body } = &mut changed_fn.ents[3].kind else {
            unreachable!()
        };
        *body = Some("{𝕨-𝕩}".into());
        assert!(migrate(&old, &changed_fn, &aliases, &manifest, &plan()).is_err());
    }

    // Migration is an introduction path for relationship storage too. The final seal admits an
    // ordinary negative dead key when its declared int carrier does, but rejects carrier-invalid
    // keyed and unkeyed targets before any outcome can be published.
    #[test]
    fn migration_seals_every_new_relationship_target() {
        let old = Registry {
            n: 2,
            ..Registry::default()
        };
        let manifest = SchemaManifest::for_registry(&old);
        let aliases = AliasEnvironment::for_registry(&old);

        let mut int_key = col("Key", ColType::Int, &[-3.0, 0.0]);
        let RegEntryKind::Col { uniq, .. } = &mut int_key.kind else {
            unreachable!()
        };
        *uniq = true;
        let admitted = Registry {
            n: 2,
            ents: vec![
                int_key,
                RegEntry {
                    name: "Link".into(),
                    defval: -1.0,
                    kind: RegEntryKind::Rel {
                        targets: vec![-2.0, -1.0],
                        key_of: Some("Key".into()),
                    },
                },
            ],
            ..Registry::default()
        };
        let keyed_plan = MigrationPlan::parse("add Key\nadd Link\n").unwrap();
        assert!(migrate(&old, &admitted, &aliases, &manifest, &keyed_plan).is_ok());

        let mut invalid_keyed = admitted.clone();
        let RegEntryKind::Col { ty, nums, .. } = &mut invalid_keyed.ents[0].kind else {
            unreachable!()
        };
        *ty = ColType::Nat;
        *nums = vec![0.0, 1.0];
        assert!(migrate(&old, &invalid_keyed, &aliases, &manifest, &keyed_plan).is_err());

        for target in [0.5, f64::INFINITY] {
            let invalid = Registry {
                n: 2,
                ents: vec![RegEntry {
                    name: "Link".into(),
                    defval: -1.0,
                    kind: RegEntryKind::Rel {
                        targets: vec![target, -1.0],
                        key_of: None,
                    },
                }],
                ..Registry::default()
            };
            let plan = MigrationPlan::parse("add Link\n").unwrap();
            assert!(migrate(&old, &invalid, &aliases, &manifest, &plan).is_err());
        }
    }

    #[test]
    fn maps_accept_whitespace_comments_and_versions_do_not_wrap() {
        let parsed = MigrationPlan::parse(
            "# heading\n\tpreserve Id\t# stable identity\nwiden Flag Enabled # carrier\npreserve Parent\npreserve Blend\nadd Added\n",
        )
        .unwrap();
        assert_eq!(parsed.rules.len(), 5);

        let (old, candidate) = worlds();
        let aliases = AliasEnvironment::for_registry(&old);
        let mut manifest = SchemaManifest::for_registry(&old);
        manifest.version = u64::MAX;
        assert!(
            migrate(&old, &candidate, &aliases, &manifest, &parsed)
                .unwrap_err()
                .msg
                .contains("version is exhausted")
        );
    }

    #[test]
    fn candidate_additions_admit_extended_real_num_but_not_nan() {
        let old = Registry {
            n: 1,
            ..Registry::default()
        };
        let aliases = AliasEnvironment::for_registry(&old);
        let manifest = SchemaManifest::for_registry(&old);
        let plan = MigrationPlan::parse("add Added\n").unwrap();

        let infinite = Registry {
            n: 1,
            ents: vec![col("Added", ColType::Num, &[f64::INFINITY])],
            ..Registry::default()
        };
        assert!(migrate(&old, &infinite, &aliases, &manifest, &plan).is_ok());

        let nan = Registry {
            n: 1,
            ents: vec![col("Added", ColType::Num, &[f64::NAN])],
            ..Registry::default()
        };
        assert!(migrate(&old, &nan, &aliases, &manifest, &plan).is_err());

        let refined = Registry {
            n: 1,
            ents: vec![col("Added", ColType::Int, &[f64::INFINITY])],
            ..Registry::default()
        };
        assert!(migrate(&old, &refined, &aliases, &manifest, &plan).is_err());
    }

    #[test]
    fn replay_and_save_reload_are_deterministic() {
        let (old, candidate) = worlds();
        let manifest = SchemaManifest::for_registry(&old);
        let aliases = AliasEnvironment::for_registry(&old);
        let left = migrate(&old, &candidate, &aliases, &manifest, &plan()).unwrap();
        let right = migrate(&old, &candidate, &aliases, &manifest, &plan()).unwrap();
        assert_eq!(left.receipt, right.receipt);
        assert_eq!(
            format!("{:?}", left.registry),
            format!("{:?}", right.registry)
        );
        assert_eq!(left.manifest.encode(), right.manifest.encode());

        let root = std::env::temp_dir().join(format!("ano-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let registry_path = root.join("world.reg");
        crate::registry::reg_dump(&left.registry, &registry_path.to_string_lossy()).unwrap();
        let loaded = crate::registry::reg_load(&registry_path.to_string_lossy()).unwrap();
        let schema_path = manifest_path(&registry_path.to_string_lossy());
        std::fs::write(&schema_path, left.manifest.encode()).unwrap();
        assert_eq!(
            SchemaManifest::load(&schema_path, &loaded).unwrap(),
            left.manifest
        );
        let alias_path = crate::alias::sidecar_path(&registry_path.to_string_lossy());
        left.aliases.save(&alias_path).unwrap();
        AliasEnvironment::load(&alias_path, &loaded).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
