use std::collections::HashSet;
use std::ops::Range;

use kfind_morph::FinePos;
use kfind_query::ContextRequirement;

use super::MorphMatcher;
use crate::{AnalysisWindow, AnalysisWindowError, DEFAULT_ANALYSIS_WINDOW_LIMITS};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAnalysisCandidate {
    pub context_requirement: ContextRequirement,
    pub atom_index: usize,
    pub analysis_index: u16,
    pub fine_pos: FinePos,
    pub target: Range<usize>,
    pub window: Result<AnalysisWindow, AnalysisWindowError>,
}

impl MorphMatcher {
    #[must_use]
    pub fn local_analysis_candidates(&self, haystack: &[u8]) -> Vec<LocalAnalysisCandidate> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for hit in self.anchor_engine.hits(haystack, 0) {
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.branches[branch_ref.branch_index];
                if branch.context_requirement == ContextRequirement::None {
                    continue;
                }
                let Some(candidate) = self.verify_branch_without_boundary(
                    haystack,
                    &hit,
                    branch,
                    super::MatchMetadata::Provenance,
                ) else {
                    continue;
                };
                let boundary_accepted = self.accepts_token_boundary(haystack, &candidate, branch);
                if boundary_accepted
                    != (branch.context_requirement == ContextRequirement::EojeolLattice)
                {
                    continue;
                }
                for origin in &candidate.origins {
                    let Some(analysis) = atom.analyses.get(usize::from(origin.analysis_index))
                    else {
                        continue;
                    };
                    let key = (
                        branch_ref.atom_index,
                        origin.analysis_index,
                        candidate.core.start,
                        candidate.core.end,
                        analysis.fine_pos,
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    candidates.push(LocalAnalysisCandidate {
                        context_requirement: branch.context_requirement,
                        atom_index: branch_ref.atom_index,
                        analysis_index: origin.analysis_index,
                        fine_pos: analysis.fine_pos,
                        target: candidate.core.clone(),
                        window: AnalysisWindow::extract(
                            haystack,
                            candidate.core.clone(),
                            DEFAULT_ANALYSIS_WINDOW_LIMITS,
                        ),
                    });
                }
            }
        }
        candidates
    }
}
