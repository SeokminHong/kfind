use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::io;

use grep_matcher::{LineMatchKind, LineTerminator, Match, Matcher, NoCaptures};
use kfind_matcher::{MatchLimitExceeded, MorphMatcher, SearchDiagnostics};
use kfind_query::PhraseMatch;

use super::InputSearchError;

const MAX_MATCHES_PER_LINE: usize = 65_536;

pub(super) struct LineMatcher<'a> {
    diagnostics: SearchDiagnostics,
    matcher: &'a MorphMatcher,
    handoff_metadata: bool,
    pending: RefCell<Option<LineEvaluation>>,
}

struct LineEvaluation {
    input_len: usize,
    matches: Vec<PhraseMatch>,
    routes: Vec<Vec<usize>>,
}

#[derive(Debug)]
pub(super) enum LineMatchError {
    MatchLimitExceeded { limit: usize },
    StateBorrowed,
    UnderlyingMatcher,
}

impl<'a> LineMatcher<'a> {
    pub(super) fn new(matcher: &'a MorphMatcher, capture_records: bool) -> Self {
        Self {
            diagnostics: SearchDiagnostics::default(),
            matcher,
            handoff_metadata: capture_records
                && Matcher::line_terminator(matcher) == Some(LineTerminator::byte(b'\n')),
            pending: RefCell::new(None),
        }
    }

    pub(super) fn structural_verification_incomplete(&self) -> bool {
        self.diagnostics.structural_verification_incomplete()
    }

    pub(super) fn take_line_matches(
        &self,
        bytes: &[u8],
    ) -> Result<(Vec<PhraseMatch>, Vec<Vec<usize>>), InputSearchError> {
        if !self.handoff_metadata {
            return self
                .evaluate(bytes)
                .map(|evaluation| (evaluation.matches, evaluation.routes))
                .map_err(|error| InputSearchError::MatchLimitExceeded {
                    limit: error.limit(),
                });
        }
        let mut pending = self
            .pending
            .try_borrow_mut()
            .map_err(|_| line_evaluation_state_error())?;
        let evaluation = pending.take().ok_or_else(line_evaluation_state_error)?;
        if evaluation.input_len != line_without_terminator(bytes).len() {
            return Err(line_evaluation_state_error());
        }
        Ok((evaluation.matches, evaluation.routes))
    }

    fn evaluate(&self, bytes: &[u8]) -> Result<LineEvaluation, MatchLimitExceeded> {
        if self.matcher.plan().graph.is_some() {
            let routed = self.matcher.find_all_with_route_limit_and_diagnostics(
                bytes,
                MAX_MATCHES_PER_LINE,
                &self.diagnostics,
            )?;
            let (matches, routes) = routed
                .into_iter()
                .map(|route| (route.matched, route.query_atom_indices))
                .unzip();
            return Ok(LineEvaluation {
                input_len: bytes.len(),
                matches,
                routes,
            });
        }
        let matches = self.matcher.find_all_with_meta_limit_and_diagnostics(
            bytes,
            MAX_MATCHES_PER_LINE,
            &self.diagnostics,
        )?;
        Ok(LineEvaluation {
            input_len: bytes.len(),
            matches,
            routes: Vec::new(),
        })
    }

    fn replace_pending(&self, evaluation: Option<LineEvaluation>) -> Result<(), LineMatchError> {
        let mut pending = self
            .pending
            .try_borrow_mut()
            .map_err(|_| LineMatchError::StateBorrowed)?;
        *pending = evaluation;
        Ok(())
    }
}

impl Matcher for LineMatcher<'_> {
    type Captures = NoCaptures;
    type Error = LineMatchError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        if !self.handoff_metadata || at != 0 {
            self.replace_pending(None)?;
            return Ok(self
                .matcher
                .find_span_at_with_diagnostics(haystack, at, &self.diagnostics)
                .map(|span| Match::new(span.start, span.end)));
        }
        let evaluation =
            self.evaluate(haystack)
                .map_err(|error| LineMatchError::MatchLimitExceeded {
                    limit: error.limit(),
                })?;
        let first = evaluation
            .matches
            .first()
            .map(|matched| Match::new(matched.span.start, matched.span.end));
        self.replace_pending(first.map(|_| evaluation))?;
        Ok(first)
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        Matcher::line_terminator(self.matcher)
    }

    fn find_candidate_line(&self, haystack: &[u8]) -> Result<Option<LineMatchKind>, Self::Error> {
        Matcher::find_candidate_line(self.matcher, haystack)
            .map_err(|_| LineMatchError::UnderlyingMatcher)
    }
}

impl Display for LineMatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MatchLimitExceeded { limit } => {
                write!(formatter, "matches per line exceed limit {limit}")
            }
            Self::StateBorrowed => formatter.write_str("line evaluation state is already borrowed"),
            Self::UnderlyingMatcher => formatter.write_str("underlying matcher failed"),
        }
    }
}

#[cfg(test)]
pub(super) fn collect_line_matches(
    matcher: &MorphMatcher,
    bytes: &[u8],
    limit: usize,
) -> Result<Vec<PhraseMatch>, InputSearchError> {
    matcher
        .find_all_with_meta_limit(bytes, limit)
        .map_err(|error| InputSearchError::MatchLimitExceeded {
            limit: error.limit(),
        })
}

fn line_without_terminator(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

fn line_evaluation_state_error() -> InputSearchError {
    InputSearchError::Io(io::Error::other("line evaluation state mismatch"))
}
