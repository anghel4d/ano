//! Deterministic semantic-use metadata for relationship and fiber diagnostics.
//!
//! One record per RUNTIME CROSSING, minted by the lowerer at the moment the crossing is
//! staged.  Ids are monotone in staging order, which is source evaluation order.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceUseId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePhase {
    Predicate,
    Effect,
}

impl TracePhase {
    pub fn word(self) -> &'static str {
        match self {
            TracePhase::Predicate => "PREDICATE",
            TracePhase::Effect => "EFFECT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDomain {
    Source,
    Selected,
}

impl TraceDomain {
    pub fn word(self) -> &'static str {
        match self {
            TraceDomain::Source => "SOURCE",
            TraceDomain::Selected => "SELECTED",
        }
    }
}

// site is the emission block the crossing belongs to (s<N>/q<N>/c<N>); line is its 1-based
// source line.  domain is Selected exactly when the statement mask was conjoined into the
// reported mask, so the record and the emitted mask can never disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceUse {
    pub id: TraceUseId,
    pub phase: TracePhase,
    pub domain: TraceDomain,
    pub site: String,
    pub line: u32,
    pub relation: String,
}

#[derive(Debug, Clone, Default)]
pub struct TracePlan {
    uses: Vec<TraceUse>,
    aliases: Vec<String>,
}

impl TracePlan {
    // Inputs: the phase and domain the staging site established, its site/line, the relation.
    // Output: the id the runtime line carries.  Invariant: id == position in the plan.
    pub fn record(
        &mut self,
        phase: TracePhase,
        domain: TraceDomain,
        site: impl Into<String>,
        line: u32,
        relation: impl Into<String>,
    ) -> TraceUseId {
        let id = TraceUseId(self.uses.len() as u64);
        self.uses.push(TraceUse {
            id,
            phase,
            domain,
            site: site.into(),
            line,
            relation: relation.into(),
        });
        id
    }

    pub fn uses(&self) -> &[TraceUse] {
        &self.uses
    }

    pub fn len(&self) -> usize {
        self.uses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.uses.is_empty()
    }

    // The discarded guard probe stages nothing; it must record nothing either.
    pub fn truncate(&mut self, len: usize) {
        self.uses.truncate(len);
    }

    // One dynamic-alias consultation, already spelled, in consultation order. Recording is
    // unconditional; the trace flag gates only whether the lines reach the emitted plan.
    pub fn record_alias(&mut self, line: impl Into<String>) {
        self.aliases.push(line.into());
    }

    pub fn alias_lines(&self) -> &[String] {
        &self.aliases
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_are_recorded_without_deduplication() {
        let mut plan = TracePlan::default();
        plan.record(TracePhase::Predicate, TraceDomain::Source, "s1", 3, "parent");
        plan.record(TracePhase::Effect, TraceDomain::Selected, "s1", 3, "parent");
        assert_eq!(plan.uses().len(), 2);
        assert_eq!(plan.uses()[0].domain, TraceDomain::Source);
        assert_eq!(plan.uses()[1].domain, TraceDomain::Selected);
        assert_ne!(plan.uses()[0].id, plan.uses()[1].id);
    }

    #[test]
    fn truncation_restores_the_id_sequence() {
        let mut plan = TracePlan::default();
        plan.record(TracePhase::Effect, TraceDomain::Selected, "s1", 1, "a");
        let mark = plan.len();
        plan.record(TracePhase::Effect, TraceDomain::Selected, "s1", 1, "b");
        plan.truncate(mark);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan.record(TracePhase::Effect, TraceDomain::Selected, "s1", 1, "c").0, 1);
    }
}
