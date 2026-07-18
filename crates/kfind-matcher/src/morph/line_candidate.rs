use grep_matcher::LineMatchKind;
use memchr::memchr;

use super::{MorphMatcher, ProgramRef};

pub(super) fn find(matcher: &MorphMatcher, haystack: &[u8]) -> Option<LineMatchKind> {
    if !matcher.is_line_local || matcher.plan.atoms.len() == 1 {
        return first_anchor(matcher, haystack);
    }

    let mut hits = matcher.anchor_engine.hits(haystack, 0);
    let mut current = hits.next()?;
    let mut covered_atoms = vec![false; matcher.plan.atoms.len()];

    loop {
        covered_atoms.fill(false);
        let mut remaining_atoms = covered_atoms.len();
        let candidate_position = current.span.start;
        let line_end = memchr(b'\n', &haystack[candidate_position..])
            .map_or(haystack.len(), |relative| candidate_position + relative);

        loop {
            cover_atoms(
                &mut covered_atoms,
                &matcher.anchor_programs[current.anchor_index],
                &mut remaining_atoms,
            );
            if remaining_atoms == 0 {
                return Some(LineMatchKind::Candidate(candidate_position));
            }

            let next = hits.next()?;
            if next.span.start >= line_end {
                current = next;
                break;
            }
            current = next;
        }
    }
}

fn first_anchor(matcher: &MorphMatcher, haystack: &[u8]) -> Option<LineMatchKind> {
    matcher
        .anchor_engine
        .hits(haystack, 0)
        .next()
        .map(|hit| LineMatchKind::Candidate(hit.span.start))
}

fn cover_atoms(covered: &mut [bool], programs: &[ProgramRef], remaining: &mut usize) {
    for program in programs {
        let atom = &mut covered[program.atom_index];
        if !*atom {
            *atom = true;
            *remaining -= 1;
        }
    }
}
