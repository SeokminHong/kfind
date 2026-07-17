use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

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
    anchor: Box<[u8]>,
    consumption: CandidateConsumption,
    core_mapping: CoreMapping,
    boundary: BoundaryProof,
    decision: DraftDecision,
}

struct MergedProgram {
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
    for draft in drafts {
        let allow_attached = matches!(
            draft.consumption,
            CandidateConsumption::DirectParticleHost { .. }
        );
        let boundary_proof =
            boundary_proof(boundary, draft.smart_left, one_scalar_atom, allow_attached);
        let decision = candidate_decision(boundary, draft.decision);
        'forms: for (anchor, core_mapping) in normalized_forms(&draft, mode, atom_index)? {
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
                        for origin in &draft.origins {
                            if !origins.contains(origin) {
                                origins.push(origin.clone());
                            }
                        }
                        origins.sort();
                        continue 'forms;
                    }
                    current = merged[index].next_same_anchor;
                }

                let index = merged.len();
                merged.push(MergedProgram {
                    key: ProgramKey {
                        anchor: anchor.into_bytes().into_boxed_slice(),
                        consumption: draft.consumption.clone(),
                        core_mapping,
                        boundary: boundary_proof,
                        decision,
                    },
                    origins: draft.origins.clone(),
                    next_same_anchor: Some(previous_head),
                });
                *head = index;
            } else {
                let index = merged.len();
                let anchor = anchor.into_bytes().into_boxed_slice();
                anchor_heads.insert(anchor.clone(), index);
                merged.push(MergedProgram {
                    key: ProgramKey {
                        anchor,
                        consumption: draft.consumption.clone(),
                        core_mapping,
                        boundary: boundary_proof,
                        decision,
                    },
                    origins: draft.origins.clone(),
                    next_same_anchor: None,
                });
            }
        }
    }
    Ok(merged
        .into_iter()
        .map(|merged| materialize_program(merged, analyses))
        .collect())
}

fn candidate_decision(policy: BoundaryPolicy, requested: DraftDecision) -> DraftDecision {
    if policy == BoundaryPolicy::Smart {
        requested
    } else {
        DraftDecision::Boundary
    }
}

fn materialize_program(merged: MergedProgram, analyses: &[Analysis]) -> CandidateProgram {
    let MergedProgram { key, origins, .. } = merged;
    let mut program = CandidateProgram {
        anchor: key.anchor,
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

fn normalized_forms(
    draft: &DraftBranch,
    mode: NormalizationMode,
    atom_index: usize,
) -> Result<Vec<(String, CoreMapping)>, CompileError> {
    let forms = match mode {
        NormalizationMode::None => vec![(draft.anchor.clone(), NormalizedForm::Raw)],
        NormalizationMode::Nfc => vec![(draft.anchor.nfc().collect(), NormalizedForm::Nfc)],
        NormalizationMode::Canonical => {
            let nfc = draft.anchor.nfc().collect::<String>();
            let nfd = draft.anchor.nfd().collect::<String>();
            if nfc == nfd {
                vec![(nfc, NormalizedForm::Nfc)]
            } else {
                vec![(nfc, NormalizedForm::Nfc), (nfd, NormalizedForm::Nfd)]
            }
        }
    };
    forms
        .into_iter()
        .map(|(anchor, form)| {
            let mapping = match draft.core_mapping {
                CoreMapping::WholeAnchor => CoreMapping::WholeAnchor,
                CoreMapping::PrefixBytes(length) => {
                    let prefix = draft.anchor.get(..length).ok_or_else(|| {
                        CompileError::new(Some(atom_index), CompileErrorKind::InvalidCoreMapping)
                    })?;
                    let normalized_length = match form {
                        NormalizedForm::Raw => prefix.len(),
                        NormalizedForm::Nfc => prefix.nfc().collect::<String>().len(),
                        NormalizedForm::Nfd => prefix.nfd().collect::<String>().len(),
                    };
                    CoreMapping::PrefixBytes(normalized_length)
                }
            };
            Ok((anchor, mapping))
        })
        .collect()
}

#[derive(Clone, Copy)]
enum NormalizedForm {
    Raw,
    Nfc,
    Nfd,
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
