use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

use crate::{
    Analysis, BoundaryPolicy, BoundaryProof, CandidateConsumption, CandidateDecision,
    CandidateExtentPolicy, CandidateProgram, CompileError, CompileErrorKind, CoreMapping,
    NormalizationMode, Origin, QueryAtom,
};
use kfind_morph::ComponentCapability;

#[derive(Clone)]
pub(super) struct DraftBranch {
    pub anchor: String,
    pub consumption: CandidateConsumption,
    pub core_mapping: CoreMapping,
    pub extent: CandidateExtentPolicy,
    pub origin: Origin,
    pub smart_left: bool,
    pub decision: DraftDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum DraftDecision {
    Boundary,
    Structural(ComponentCapability),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProgramKey {
    anchor: Box<[u8]>,
    consumption: CandidateConsumption,
    core_mapping: CoreMapping,
    extent: CandidateExtentPolicy,
    boundary: BoundaryProof,
    decision: DraftDecision,
}

struct MergedProgram {
    key: ProgramKey,
    origins: Vec<Origin>,
}

pub(super) fn normalize_and_merge(
    drafts: Vec<DraftBranch>,
    analyses: &[Analysis],
    mode: NormalizationMode,
    boundary: BoundaryPolicy,
    one_scalar_atom: bool,
    atom_index: usize,
) -> Result<Vec<CandidateProgram>, CompileError> {
    let mut indices = HashMap::<ProgramKey, usize>::new();
    let mut merged = Vec::<MergedProgram>::new();
    for draft in drafts {
        for (anchor, core_mapping) in normalized_forms(&draft, mode, atom_index)? {
            let allow_attached = matches!(
                draft.consumption,
                CandidateConsumption::DirectParticleHost { .. }
            );
            let boundary_proof =
                boundary_proof(boundary, draft.smart_left, one_scalar_atom, allow_attached);
            let decision = candidate_decision(boundary, draft.decision);
            let key = ProgramKey {
                anchor: anchor.as_bytes().into(),
                consumption: draft.consumption.clone(),
                core_mapping,
                extent: draft.extent,
                boundary: boundary_proof,
                decision,
            };
            if let Some(index) = indices.get(&key).copied() {
                let origins = &mut merged[index].origins;
                if !origins.contains(&draft.origin) {
                    origins.push(draft.origin.clone());
                    origins.sort();
                }
            } else {
                let index = merged.len();
                indices.insert(key.clone(), index);
                merged.push(MergedProgram {
                    key,
                    origins: vec![draft.origin.clone()],
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
    let MergedProgram { key, origins } = merged;
    let mut program = CandidateProgram {
        anchor: key.anchor,
        core_mapping: key.core_mapping,
        extent: key.extent,
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
