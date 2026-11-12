use std::collections::HashMap;

use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::{
    Analysis, BoundaryPolicy, BoundaryProof, CandidateConsumption, CandidateDecision,
    CandidateProgram, CompileError, CompileErrorKind, CoreMapping, NormalizationMode, Origin,
    QueryAtom,
};
use kfind_morph::ComponentCapability;

#[derive(Clone)]
pub(super) struct DraftBranch {
    pub anchor: String,
    pub consumption: CandidateConsumption,
    pub core_mapping: CoreMapping,
    pub origins: Vec<Origin>,
    pub smart_left: bool,
    pub decision: DraftDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum DraftDecision {
    Boundary,
    Structural(ComponentCapability),
}

#[derive(Debug)]
struct ProgramKey {
    consumption: CandidateConsumption,
    core_mapping: CoreMapping,
    boundary: BoundaryProof,
    decision: DraftDecision,
}

struct MergedProgram {
    anchor: Option<Box<[u8]>>,
    key: ProgramKey,
    origins: Vec<Origin>,
    next_same_anchor: Option<usize>,
}

impl ProgramKey {
    fn matches(
        &self,
        consumption: &CandidateConsumption,
        core_mapping: CoreMapping,
        boundary: BoundaryProof,
        decision: DraftDecision,
    ) -> bool {
        self.consumption.merge_compatible(consumption)
            && self.core_mapping == core_mapping
            && self.boundary == boundary
            && self.decision == decision
    }
}

pub(super) fn normalize_and_merge(
    drafts: Vec<DraftBranch>,
    analyses: &[Analysis],
    mode: NormalizationMode,
    boundary: BoundaryPolicy,
    one_scalar_atom: bool,
    atom_index: usize,
) -> Result<Vec<CandidateProgram>, CompileError> {
    let form_capacity = drafts
        .len()
        .saturating_mul(if mode == NormalizationMode::Canonical {
            2
        } else {
            1
        });
    let mut anchor_heads = HashMap::<Box<[u8]>, usize>::with_capacity(form_capacity);
    let mut merged = Vec::<MergedProgram>::with_capacity(form_capacity);
    for mut draft in drafts {
        let allow_attached = matches!(
            draft.consumption,
            CandidateConsumption::DirectParticleHost { .. }
        );
        let boundary_proof =
            boundary_proof(boundary, draft.smart_left, one_scalar_atom, allow_attached);
        let decision = candidate_decision(boundary, draft.decision);
        let (first, second) = normalized_forms(
            std::mem::take(&mut draft.anchor),
            draft.core_mapping,
            mode,
            atom_index,
        )?;
        let has_second = second.is_some();
        let forms = std::iter::once(first).chain(second);
        'forms: for (form_index, (anchor, core_mapping)) in forms.enumerate() {
            let last_form = !has_second || form_index == 1;
            if let Some(head) = anchor_heads.get_mut(anchor.as_bytes()) {
                let previous_head = *head;
                let mut current = Some(previous_head);
                while let Some(index) = current {
                    if merged[index].key.matches(
                        &draft.consumption,
                        core_mapping,
                        boundary_proof,
                        decision,
                    ) {
                        let program = &mut merged[index];
                        program
                            .key
                            .consumption
                            .merge_source_positions(&draft.consumption);
                        let origins = &mut program.origins;
                        if last_form {
                            for origin in draft.origins.drain(..) {
                                if !origins.contains(&origin) {
                                    origins.push(origin);
                                }
                            }
                        } else {
                            for origin in &draft.origins {
                                if !origins.contains(origin) {
                                    origins.push(origin.clone());
                                }
                            }
                        }
                        origins.sort();
                        continue 'forms;
                    }
                    current = merged[index].next_same_anchor;
                }

                let (consumption, origins) = draft.metadata_for_form(last_form);
                let index = merged.len();
                merged.push(MergedProgram {
                    anchor: None,
                    key: ProgramKey {
                        consumption,
                        core_mapping,
                        boundary: boundary_proof,
                        decision,
                    },
                    origins,
                    next_same_anchor: Some(previous_head),
                });
                *head = index;
            } else {
                let (consumption, origins) = draft.metadata_for_form(last_form);
                let index = merged.len();
                let anchor = anchor.into_bytes().into_boxed_slice();
                anchor_heads.insert(anchor, index);
                merged.push(MergedProgram {
                    anchor: None,
                    key: ProgramKey {
                        consumption,
                        core_mapping,
                        boundary: boundary_proof,
                        decision,
                    },
                    origins,
                    next_same_anchor: None,
                });
            }
        }
    }
    for (anchor, mut index) in anchor_heads {
        while let Some(next) = merged[index].next_same_anchor {
            merged[index].anchor = Some(anchor.clone());
            index = next;
        }
        merged[index].anchor = Some(anchor);
    }
    Ok(merged
        .into_iter()
        .map(|merged| materialize_program(merged, analyses))
        .collect())
}

impl DraftBranch {
    fn metadata_for_form(&mut self, last_form: bool) -> (CandidateConsumption, Vec<Origin>) {
        if last_form {
            (
                std::mem::replace(&mut self.consumption, CandidateConsumption::Anchor),
                std::mem::take(&mut self.origins),
            )
        } else {
            (self.consumption.clone(), self.origins.clone())
        }
    }
}

fn candidate_decision(policy: BoundaryPolicy, requested: DraftDecision) -> DraftDecision {
    if policy == BoundaryPolicy::Smart {
        requested
    } else {
        DraftDecision::Boundary
    }
}

fn materialize_program(merged: MergedProgram, analyses: &[Analysis]) -> CandidateProgram {
    let MergedProgram {
        anchor,
        key,
        origins,
        ..
    } = merged;
    let mut program = CandidateProgram {
        anchor: anchor.expect("each program belongs to an anchor group"),
        core_mapping: key.core_mapping,
        consumption: key.consumption,
        origins,
        decision: CandidateDecision::Boundary(key.boundary),
    };
    if let DraftDecision::Structural(component_capability) = key.decision {
        program.apply_structural_constraint(analyses, component_capability);
    }
    program
}

fn boundary_proof(
    policy: BoundaryPolicy,
    smart_left: bool,
    one_scalar_anchor: bool,
    allow_attached: bool,
) -> BoundaryProof {
    match policy {
        BoundaryPolicy::Any => BoundaryProof {
            require_left: false,
            require_right: false,
            one_scalar_anchor,
        },
        BoundaryPolicy::Token => BoundaryProof {
            require_left: true,
            require_right: true,
            one_scalar_anchor,
        },
        BoundaryPolicy::Smart => BoundaryProof {
            require_left: smart_left || (one_scalar_anchor && !allow_attached),
            require_right: true,
            one_scalar_anchor,
        },
    }
}

type NormalizedAnchor = (String, CoreMapping);

fn normalized_forms(
    anchor: String,
    core_mapping: CoreMapping,
    mode: NormalizationMode,
    atom_index: usize,
) -> Result<(NormalizedAnchor, Option<NormalizedAnchor>), CompileError> {
    let prefix = match core_mapping {
        CoreMapping::WholeAnchor => None,
        CoreMapping::PrefixBytes(length) => Some(anchor.get(..length).ok_or_else(|| {
            CompileError::new(Some(atom_index), CompileErrorKind::InvalidCoreMapping)
        })?),
    };
    if mode == NormalizationMode::None {
        return Ok(((anchor, core_mapping), None));
    }
    let decomposed = (mode == NormalizationMode::Canonical).then(|| {
        let mapping = prefix.map_or(CoreMapping::WholeAnchor, |prefix| {
            CoreMapping::PrefixBytes(prefix.nfd().map(char::len_utf8).sum())
        });
        (anchor.nfd().collect::<String>(), mapping)
    });
    let composed = if is_nfc(&anchor) {
        (anchor, core_mapping)
    } else {
        let mapping = prefix.map_or(CoreMapping::WholeAnchor, |prefix| {
            CoreMapping::PrefixBytes(prefix.nfc().map(char::len_utf8).sum())
        });
        (anchor.nfc().collect(), mapping)
    };
    let decomposed = decomposed.filter(|(anchor, _)| *anchor != composed.0);
    Ok((composed, decomposed))
}

pub(super) fn normalize_atom(atom: &QueryAtom, mode: NormalizationMode) -> QueryAtom {
    let raw = match mode {
        NormalizationMode::None => atom.raw.clone(),
        NormalizationMode::Nfc | NormalizationMode::Canonical => {
            atom.raw.nfc().collect::<String>().into_boxed_str()
        }
    };
    QueryAtom {
        raw,
        forced_pos: atom.forced_pos,
        quoted_literal: atom.quoted_literal,
    }
}
