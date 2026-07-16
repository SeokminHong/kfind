use std::collections::HashSet;
use std::ops::Range;

use kfind_morph::{FinePos, RuleId};
use kfind_query::CandidateDecision;

use super::MorphMatcher;
use crate::{AnalysisWindow, AnalysisWindowError, DEFAULT_ANALYSIS_WINDOW_LIMITS};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAnalysisCandidate {
    pub atom_index: usize,
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
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
            for branch_ref in &self.anchor_programs[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.programs[branch_ref.program_index];
                if !matches!(&branch.decision, CandidateDecision::Structural(_)) {
                    continue;
                }
                let Some(candidate) = self.execute_program_without_decision(
                    haystack,
                    &hit,
                    branch,
                    super::MatchMetadata::Provenance,
                ) else {
                    continue;
                };
                if self.accepts_token_boundary(haystack, &candidate.verified, branch) {
                    continue;
                }
                for origin in &candidate.verified.origins {
                    let Some(analysis) = atom.analyses.get(usize::from(origin.analysis_index))
                    else {
                        continue;
                    };
                    let key = (
                        branch_ref.atom_index,
                        origin.analysis_index,
                        candidate.verified.core.start,
                        candidate.verified.core.end,
                        origin.rule_path.clone(),
                        analysis.fine_pos,
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    candidates.push(LocalAnalysisCandidate {
                        atom_index: branch_ref.atom_index,
                        analysis_index: origin.analysis_index,
                        rule_path: origin.rule_path.clone(),
                        fine_pos: analysis.fine_pos,
                        target: candidate.verified.core.clone(),
                        window: AnalysisWindow::extract(
                            haystack,
                            candidate.verified.core.clone(),
                            DEFAULT_ANALYSIS_WINDOW_LIMITS,
                        ),
                    });
                }
            }
        }
        candidates
    }
}
