use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

use crate::{
    BoundaryPolicy, BoundaryProof, BranchVerifier, CompileError, CompileErrorKind,
    ContextRequirement, CoreMapping, NormalizationMode, Origin, QueryAtom, SurfaceBranch,
};

#[derive(Clone)]
pub(super) struct DraftBranch {
    pub anchor: String,
    pub verifier: BranchVerifier,
    pub core_mapping: CoreMapping,
    pub origin: Origin,
    pub smart_left: bool,
    pub context_requirement: ContextRequirement,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BranchKey {
    anchor: Box<[u8]>,
    verifier: BranchVerifier,
    core_mapping: CoreMapping,
    boundary: BoundaryProof,
    context_requirement: ContextRequirement,
}

pub(super) fn normalize_and_merge(
    drafts: Vec<DraftBranch>,
    mode: NormalizationMode,
    boundary: BoundaryPolicy,
    one_scalar_atom: bool,
    atom_index: usize,
) -> Result<Vec<SurfaceBranch>, CompileError> {
    let mut indices = HashMap::<BranchKey, usize>::new();
    let mut branches = Vec::<SurfaceBranch>::new();
    for draft in drafts {
        for (anchor, core_mapping) in normalized_forms(&draft, mode, atom_index)? {
            let allow_attached = matches!(draft.verifier, BranchVerifier::DirectParticle { .. });
            let boundary_proof =
                boundary_proof(boundary, draft.smart_left, one_scalar_atom, allow_attached);
            let context_requirement = context_requirement(boundary, draft.context_requirement);
            let key = BranchKey {
                anchor: anchor.as_bytes().into(),
                verifier: draft.verifier.clone(),
                core_mapping,
                boundary: boundary_proof,
                context_requirement,
            };
            if let Some(index) = indices.get(&key).copied() {
                let origins = &mut branches[index].origins;
                if !origins.contains(&draft.origin) {
                    origins.push(draft.origin.clone());
                    origins.sort();
                }
            } else {
                let index = branches.len();
                indices.insert(key.clone(), index);
                branches.push(SurfaceBranch {
                    anchor: key.anchor,
                    verifier: key.verifier,
                    core_mapping: key.core_mapping,
                    origins: vec![draft.origin.clone()],
                    boundary: key.boundary,
                    context_requirement: key.context_requirement,
                });
            }
        }
    }
    Ok(branches)
}

fn context_requirement(
    policy: BoundaryPolicy,
    requested: ContextRequirement,
) -> ContextRequirement {
    if policy == BoundaryPolicy::Smart && requested == ContextRequirement::NominalComponent {
        ContextRequirement::NominalComponent
    } else {
        ContextRequirement::None
    }
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
