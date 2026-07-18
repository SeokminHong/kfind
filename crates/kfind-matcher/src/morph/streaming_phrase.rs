use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::rc::Rc;

use kfind_query::{PhraseMatch, PhrasePolicy, VerifiedSpan};

use crate::{AnchorHit, AnchorHits};

use super::phrase::{PhraseMatchLimit, PhraseSelection};
use super::{
    MatchMetadata, MorphMatcher, StructuralCache, merge_origins, phrase_join_text, same_span,
};

pub(super) fn select(
    matcher: &MorphMatcher,
    haystack: &[u8],
    at: usize,
    metadata: MatchMetadata,
    limit: PhraseMatchLimit,
) -> PhraseSelection {
    let text = phrase_join_text(haystack);
    let mut candidates = CandidateStream::new(matcher, haystack, at, metadata);
    let mut group = Vec::new();
    let mut metrics = MetricCursor::new(&text);
    let mut selector = StreamingSelector::new(
        matcher.plan.atoms.len(),
        matcher.plan.phrase_policy,
        limit,
        at,
    );

    while let Some(group_start) = candidates.next_group(&mut group) {
        let start = metrics.advance_to(group_start);
        selector.advance_to(start);
        if selector.settle() {
            return selector.finish();
        }
        for candidate in group.drain(..) {
            let Some(token) = text.get(candidate.span.token.clone()) else {
                continue;
            };
            let end = start.advance(token);
            selector.push(candidate, end);
        }
        selector.finish_group(&text, group_start, start);
        if selector.settle() {
            return selector.finish();
        }
    }

    selector.clear_active();
    selector.settle();
    selector.finish()
}

struct CandidateStream<'matcher, 'haystack> {
    matcher: &'matcher MorphMatcher,
    haystack: &'haystack [u8],
    hits: AnchorHits<'matcher, 'haystack>,
    next_hit: Option<AnchorHit>,
    pending: Vec<PendingCandidate>,
    group: Vec<PendingCandidate>,
    next_sequence: Vec<usize>,
    structural_cache: StructuralCache,
    metadata: MatchMetadata,
}

impl<'matcher, 'haystack> CandidateStream<'matcher, 'haystack> {
    fn new(
        matcher: &'matcher MorphMatcher,
        haystack: &'haystack [u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Self {
        let mut hits = matcher.anchor_engine.hits(haystack, at);
        let next_hit = hits.next();
        Self {
            matcher,
            haystack,
            hits,
            next_hit,
            pending: Vec::new(),
            group: Vec::new(),
            next_sequence: vec![0; matcher.plan.atoms.len()],
            structural_cache: StructuralCache::default(),
            metadata,
        }
    }

    fn next_group(&mut self, candidates: &mut Vec<IndexedCandidate>) -> Option<usize> {
        candidates.clear();
        loop {
            // Anchor hits are end-ordered. Once the next hit ends beyond this window,
            // no longer anchor can produce an earlier token start.
            if let Some(start) = self
                .pending
                .iter()
                .map(|candidate| candidate.span.token.start)
                .min()
                && self.next_hit.as_ref().is_none_or(|hit| {
                    hit.span.end > start.saturating_add(self.matcher.max_anchor_bytes)
                })
            {
                self.take_group(start, candidates);
                return Some(start);
            }

            let hit = self.next_hit.take()?;
            self.next_hit = self.hits.next();
            for branch_ref in &self.matcher.anchor_programs[hit.anchor_index] {
                let branch = &self.matcher.plan.atoms[branch_ref.atom_index].programs
                    [branch_ref.program_index];
                if let Some(span) = self.matcher.execute_program_with_metadata(
                    self.haystack,
                    &hit,
                    branch,
                    self.metadata,
                    &mut self.structural_cache,
                ) {
                    self.pending.push(PendingCandidate {
                        atom_index: branch_ref.atom_index,
                        span,
                    });
                }
            }
        }
    }

    fn take_group(&mut self, start: usize, candidates: &mut Vec<IndexedCandidate>) {
        self.group.clear();
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].span.token.start == start {
                self.group.push(self.pending.swap_remove(index));
            } else {
                index += 1;
            }
        }
        self.group.sort_by_key(|candidate| {
            (
                candidate.atom_index,
                candidate.span.token.end,
                candidate.span.core.start,
                candidate.span.core.end,
            )
        });

        for candidate in self.group.drain(..) {
            if let Some(previous) = candidates.last_mut().filter(|previous| {
                previous.atom_index == candidate.atom_index
                    && same_span(&previous.span, &candidate.span)
            }) {
                merge_origins(&mut previous.span.origins, candidate.span.origins);
            } else {
                let sequence = self.next_sequence[candidate.atom_index];
                self.next_sequence[candidate.atom_index] += 1;
                candidates.push(IndexedCandidate {
                    atom_index: candidate.atom_index,
                    sequence,
                    span: candidate.span,
                });
            }
        }
    }
}

struct PendingCandidate {
    atom_index: usize,
    span: VerifiedSpan,
}

#[derive(Clone)]
struct IndexedCandidate {
    atom_index: usize,
    sequence: usize,
    span: VerifiedSpan,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct Position {
    line_breaks: usize,
    scalars: usize,
}

impl Position {
    fn advance(mut self, text: &str) -> Self {
        for character in text.chars() {
            self.scalars += 1;
            self.line_breaks += usize::from(matches!(character, '\n' | '\r'));
        }
        self
    }
}

struct MetricCursor<'text> {
    text: &'text str,
    byte: usize,
    position: Position,
}

impl<'text> MetricCursor<'text> {
    fn new(text: &'text str) -> Self {
        Self {
            text,
            byte: 0,
            position: Position::default(),
        }
    }

    fn advance_to(&mut self, byte: usize) -> Position {
        debug_assert!(byte >= self.byte);
        if let Some(text) = self.text.get(self.byte..byte) {
            self.position = self.position.advance(text);
            self.byte = byte;
        }
        self.position
    }
}

struct StreamingSelector {
    policy: PhrasePolicy,
    limit: PhraseMatchLimit,
    layers: Vec<ActiveLayer>,
    completed: BTreeMap<usize, PathState>,
    matches: Vec<PhraseMatch>,
    // Candidates are harvested only after a group remains beyond a pending match end.
    // They let a cursor advance recover prefixes that lost to an overlapping match.
    history: Vec<HistoryCandidate>,
    history_floor: Option<usize>,
    position: Position,
    cursor: usize,
    limit_exceeded: bool,
}

impl StreamingSelector {
    fn new(atom_count: usize, policy: PhrasePolicy, limit: PhraseMatchLimit, at: usize) -> Self {
        Self {
            policy,
            limit,
            layers: (0..atom_count.saturating_sub(1))
                .map(|_| ActiveLayer::default())
                .collect(),
            completed: BTreeMap::new(),
            matches: Vec::new(),
            history: Vec::new(),
            history_floor: None,
            position: Position::default(),
            cursor: at,
            limit_exceeded: false,
        }
    }

    fn push(&mut self, candidate: IndexedCandidate, end: Position) {
        self.push_state(candidate, end);
    }

    fn push_state(&mut self, candidate: IndexedCandidate, end: Position) {
        if candidate.atom_index == 0 {
            if candidate.span.token.start < self.cursor {
                return;
            }
            let path = PathState::one(candidate);
            if self.layers.is_empty() {
                self.record_complete(path);
            } else {
                self.layers[0].insert(end, path);
            }
            return;
        }

        let predecessor_index = candidate.atom_index - 1;
        let Some(predecessor) = self
            .layers
            .get(predecessor_index)
            .and_then(ActiveLayer::best)
        else {
            return;
        };
        let final_step = candidate.atom_index == self.layers.len();
        let path = predecessor.extend(candidate, final_step);
        if path.len() == self.layers.len() + 1 {
            self.record_complete(path);
        } else if let Some(layer) = self.layers.get_mut(path.len() - 1) {
            layer.insert(end, path);
        }
    }

    fn record_complete(&mut self, path: PathState) {
        let start = path.first_start();
        match self.completed.entry(start) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(path);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let current = entry.get();
                if path.final_end() > current.final_end()
                    || (path.final_end() == current.final_end()
                        && path.compare_precedence(current).is_lt())
                {
                    entry.insert(path);
                }
            }
        }
        self.refresh_history_floor();
    }

    fn finish_group(&mut self, text: &str, group_start: usize, start: Position) {
        if !self.history_floor.is_some_and(|floor| group_start >= floor) {
            return;
        }

        let mut candidates = Vec::new();
        for layer in &self.layers {
            layer.collect_group_candidates(group_start, &mut candidates);
        }
        for path in self.completed.values() {
            path.collect_group_candidates(group_start, &mut candidates);
        }
        candidates.sort_by_key(|candidate| (candidate.atom_index, candidate.sequence));
        candidates.dedup_by_key(|candidate| (candidate.atom_index, candidate.sequence));
        self.history
            .extend(candidates.into_iter().filter_map(|candidate| {
                let token = text.get(candidate.span.token.clone())?;
                Some(HistoryCandidate {
                    candidate,
                    start,
                    end: start.advance(token),
                })
            }));
    }

    fn advance_to(&mut self, start: Position) {
        for layer in &mut self.layers {
            layer.advance_to(start, self.policy.max_gap);
        }
        self.position = start;
    }

    fn settle(&mut self) -> bool {
        loop {
            let Some((&complete_start, _)) = self.completed.first_key_value() else {
                return false;
            };
            let active_start = self
                .layers
                .iter()
                .filter_map(ActiveLayer::minimum_first_start)
                .min();
            if active_start.is_some_and(|active| active <= complete_start) {
                return false;
            }

            let Some(path) = self.completed.remove(&complete_start) else {
                continue;
            };
            if complete_start < self.cursor {
                continue;
            }
            if matches!(self.limit, PhraseMatchLimit::Bounded(maximum) if self.matches.len() == maximum)
            {
                self.limit_exceeded = true;
                return true;
            }
            let matched = path.into_match();
            self.cursor = matched.span.end;
            self.matches.push(matched);
            if self.limit == PhraseMatchLimit::First {
                return true;
            }
            self.rebuild_from_history();
        }
    }

    fn clear_active(&mut self) {
        for layer in &mut self.layers {
            layer.clear();
        }
    }

    fn finish(self) -> PhraseSelection {
        PhraseSelection {
            matches: self.matches,
            limit_exceeded: self.limit_exceeded,
        }
    }

    fn rebuild_from_history(&mut self) {
        let current_position = self.position;
        self.history
            .retain(|candidate| candidate.candidate.span.token.start >= self.cursor);
        let replay = self.history.clone();
        for layer in &mut self.layers {
            layer.clear();
        }
        self.completed.clear();
        self.history_floor = None;

        for candidate in replay {
            self.advance_to(candidate.start);
            self.push_state(candidate.candidate, candidate.end);
        }
        self.advance_to(current_position);
        self.refresh_history_floor();
    }

    fn refresh_history_floor(&mut self) {
        let next_floor = self
            .completed
            .first_key_value()
            .map(|(_, path)| path.final_end());
        if self.history_floor == next_floor {
            return;
        }
        self.history_floor = next_floor;
        if self.history.is_empty() {
            return;
        }
        if let Some(floor) = self.history_floor {
            self.history
                .retain(|candidate| candidate.candidate.span.token.start >= floor);
        } else {
            self.history.clear();
        }
    }
}

#[derive(Clone)]
struct HistoryCandidate {
    candidate: IndexedCandidate,
    start: Position,
    end: Position,
}

#[derive(Default)]
struct ActiveLayer {
    // Token ends become eligible as the candidate-start cursor reaches them.
    future: Vec<ActivePath>,
    // The preferred compatible prefix is always at the heap root.
    eligible: BinaryHeap<ActivePath>,
    // Same-end losers do not affect the current match, but remain replay sources.
    alternates: Vec<ActivePath>,
}

impl ActiveLayer {
    fn advance_to(&mut self, start: Position, max_gap: usize) {
        let mut index = 0;
        while index < self.future.len() {
            if self.future[index].end <= start {
                let path = self.future.swap_remove(index);
                self.eligible.push(path);
            } else {
                index += 1;
            }
        }

        self.remove_expired_top(start, max_gap);
        let compact_at = max_gap.saturating_add(1).saturating_mul(2).max(64);
        if self.eligible.len() > compact_at {
            self.eligible
                .retain(|path| path.can_precede(start, max_gap));
        }
        self.alternates
            .retain(|path| path.end > start || path.can_precede(start, max_gap));
    }

    fn insert(&mut self, end: Position, path: PathState) {
        if let Some(index) = self.future.iter().position(|current| current.end == end) {
            let discarded = if path.compare_precedence(&self.future[index].path).is_lt() {
                std::mem::replace(&mut self.future[index].path, path)
            } else {
                path
            };
            self.alternates.push(ActivePath {
                end,
                path: discarded,
            });
        } else {
            self.future.push(ActivePath { end, path });
        }
    }

    fn best(&self) -> Option<&PathState> {
        self.eligible.peek().map(|path| &path.path)
    }

    fn minimum_first_start(&self) -> Option<usize> {
        self.eligible
            .peek()
            .map(|path| path.path.first_start())
            .into_iter()
            .chain(self.future.iter().map(|path| path.path.first_start()))
            .min()
    }

    fn clear(&mut self) {
        self.future.clear();
        self.eligible.clear();
        self.alternates.clear();
    }

    fn collect_group_candidates(&self, group_start: usize, output: &mut Vec<IndexedCandidate>) {
        for path in self
            .future
            .iter()
            .chain(self.eligible.iter())
            .chain(&self.alternates)
        {
            path.path.collect_group_candidates(group_start, output);
        }
    }

    fn remove_expired_top(&mut self, start: Position, max_gap: usize) {
        while self
            .eligible
            .peek()
            .is_some_and(|path| !path.can_precede(start, max_gap))
        {
            self.eligible.pop();
        }
    }
}

struct ActivePath {
    end: Position,
    path: PathState,
}

impl ActivePath {
    fn can_precede(&self, start: Position, max_gap: usize) -> bool {
        self.end.line_breaks == start.line_breaks
            && self.end.scalars.saturating_add(max_gap) >= start.scalars
    }
}

impl PartialEq for ActivePath {
    fn eq(&self, other: &Self) -> bool {
        self.end == other.end && self.path.compare_precedence(&other.path).is_eq()
    }
}

impl Eq for ActivePath {}

impl PartialOrd for ActivePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActivePath {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .path
            .compare_precedence(&self.path)
            .then_with(|| other.end.cmp(&self.end))
    }
}

struct PathStep {
    span: VerifiedSpan,
    sequence: usize,
}

enum PathState {
    One(PathStep),
    Pair(PathStep, PathStep),
    Many {
        tail: Rc<PathNode>,
        len: usize,
        first_start: usize,
        final_end: usize,
    },
}

enum PathNode {
    // Prefix nodes share verified spans instead of cloning the full prefix per extension.
    Pair(PathStep, PathStep),
    More {
        previous: Rc<PathNode>,
        step: PathStep,
    },
}

impl PathState {
    fn one(candidate: IndexedCandidate) -> Self {
        Self::One(PathStep {
            span: candidate.span,
            sequence: candidate.sequence,
        })
    }

    fn extend(&self, candidate: IndexedCandidate, final_step: bool) -> Self {
        let step = PathStep {
            span: candidate.span,
            sequence: candidate.sequence,
        };
        match self {
            Self::One(first) if final_step => Self::Pair(
                PathStep {
                    span: first.span.clone(),
                    sequence: first.sequence,
                },
                step,
            ),
            Self::One(first) => Self::Many {
                first_start: first.span.token.start,
                final_end: step.span.token.end,
                len: 2,
                tail: Rc::new(PathNode::Pair(
                    PathStep {
                        span: first.span.clone(),
                        sequence: first.sequence,
                    },
                    step,
                )),
            },
            Self::Pair(first, second) => Self::Many {
                first_start: first.span.token.start,
                final_end: step.span.token.end,
                len: 3,
                tail: Rc::new(PathNode::More {
                    previous: Rc::new(PathNode::Pair(
                        PathStep {
                            span: first.span.clone(),
                            sequence: first.sequence,
                        },
                        PathStep {
                            span: second.span.clone(),
                            sequence: second.sequence,
                        },
                    )),
                    step,
                }),
            },
            Self::Many {
                tail,
                len,
                first_start,
                ..
            } => Self::Many {
                first_start: *first_start,
                final_end: step.span.token.end,
                len: len + 1,
                tail: Rc::new(PathNode::More {
                    previous: Rc::clone(tail),
                    step,
                }),
            },
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Pair(_, _) => 2,
            Self::Many { len, .. } => *len,
        }
    }

    fn first_start(&self) -> usize {
        match self {
            Self::One(step) => step.span.token.start,
            Self::Pair(first, _) => first.span.token.start,
            Self::Many { first_start, .. } => *first_start,
        }
    }

    fn final_end(&self) -> usize {
        match self {
            Self::One(step) => step.span.token.end,
            Self::Pair(_, second) => second.span.token.end,
            Self::Many { final_end, .. } => *final_end,
        }
    }

    fn sequence(&self, index: usize) -> usize {
        match self {
            Self::One(step) => {
                debug_assert_eq!(index, 0);
                step.sequence
            }
            Self::Pair(first, second) => {
                if index == 0 {
                    first.sequence
                } else {
                    second.sequence
                }
            }
            Self::Many { tail, len, .. } => sequence_at(tail, *len, index),
        }
    }

    fn compare_precedence(&self, other: &Self) -> Ordering {
        let ordering = self.first_start().cmp(&other.first_start());
        if !ordering.is_eq() {
            return ordering;
        }
        if let (
            Self::Many {
                tail: left_tail,
                len: left_len,
                ..
            },
            Self::Many {
                tail: right_tail,
                len: right_len,
                ..
            },
        ) = (self, other)
            && left_len == right_len
        {
            return compare_node_sequences(left_tail, right_tail);
        }

        let mut ordering = Ordering::Equal;
        let compared = self.len().min(other.len());
        for index in 0..compared {
            ordering = ordering.then_with(|| self.sequence(index).cmp(&other.sequence(index)));
        }
        ordering.then_with(|| self.len().cmp(&other.len()))
    }

    fn into_match(self) -> PhraseMatch {
        let atoms = match self {
            Self::One(step) => vec![step.span],
            Self::Pair(first, second) => vec![first.span, second.span],
            Self::Many { tail, len, .. } => {
                let mut reversed = Vec::with_capacity(len);
                let mut node = tail.as_ref();
                loop {
                    match node {
                        PathNode::Pair(first, second) => {
                            reversed.push(&second.span);
                            reversed.push(&first.span);
                            break;
                        }
                        PathNode::More { previous, step } => {
                            reversed.push(&step.span);
                            node = previous.as_ref();
                        }
                    }
                }
                reversed.into_iter().rev().cloned().collect::<Vec<_>>()
            }
        };
        let start = atoms.first().map_or(0, |atom| atom.token.start);
        let end = atoms.last().map_or(start, |atom| atom.token.end);
        PhraseMatch {
            span: start..end,
            atoms,
        }
    }

    fn collect_group_candidates(&self, group_start: usize, output: &mut Vec<IndexedCandidate>) {
        match self {
            Self::One(step) => collect_step_candidate(step, 0, group_start, output),
            Self::Pair(first, second) => {
                collect_step_candidate(first, 0, group_start, output);
                collect_step_candidate(second, 1, group_start, output);
            }
            Self::Many { tail, len, .. } => {
                let mut reversed = Vec::with_capacity(*len);
                let mut node = tail.as_ref();
                loop {
                    match node {
                        PathNode::Pair(first, second) => {
                            reversed.push(second);
                            reversed.push(first);
                            break;
                        }
                        PathNode::More { previous, step } => {
                            reversed.push(step);
                            node = previous.as_ref();
                        }
                    }
                }
                for (atom_index, step) in reversed.into_iter().rev().enumerate() {
                    collect_step_candidate(step, atom_index, group_start, output);
                }
            }
        }
    }
}

fn collect_step_candidate(
    step: &PathStep,
    atom_index: usize,
    group_start: usize,
    output: &mut Vec<IndexedCandidate>,
) {
    if step.span.token.start == group_start {
        output.push(IndexedCandidate {
            atom_index,
            sequence: step.sequence,
            span: step.span.clone(),
        });
    }
}

fn compare_node_sequences(left: &PathNode, right: &PathNode) -> Ordering {
    match (left, right) {
        (PathNode::Pair(left_first, left_second), PathNode::Pair(right_first, right_second)) => {
            left_first
                .sequence
                .cmp(&right_first.sequence)
                .then_with(|| left_second.sequence.cmp(&right_second.sequence))
        }
        (
            PathNode::More {
                previous: left_previous,
                step: left_step,
            },
            PathNode::More {
                previous: right_previous,
                step: right_step,
            },
        ) => compare_node_sequences(left_previous, right_previous)
            .then_with(|| left_step.sequence.cmp(&right_step.sequence)),
        _ => Ordering::Equal,
    }
}

fn sequence_at(node: &PathNode, len: usize, index: usize) -> usize {
    debug_assert!(index < len);
    match node {
        PathNode::Pair(first, second) => {
            debug_assert_eq!(len, 2);
            if index == 0 {
                first.sequence
            } else {
                second.sequence
            }
        }
        PathNode::More { previous, step } => {
            if index + 1 == len {
                step.sequence
            } else {
                sequence_at(previous, len - 1, index)
            }
        }
    }
}

#[cfg(test)]
pub(super) fn select_verified_spans(
    text: &str,
    atom_spans: &[Vec<VerifiedSpan>],
    policy: PhrasePolicy,
    limit: PhraseMatchLimit,
) -> PhraseSelection {
    let mut candidates = atom_spans
        .iter()
        .enumerate()
        .flat_map(|(atom_index, spans)| {
            spans
                .iter()
                .cloned()
                .map(move |span| PendingCandidate { atom_index, span })
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        (
            candidate.span.token.start,
            candidate.atom_index,
            candidate.span.token.end,
            candidate.span.core.start,
            candidate.span.core.end,
        )
    });
    let mut next_sequence = vec![0; atom_spans.len()];
    let mut metrics = MetricCursor::new(text);
    let mut selector = StreamingSelector::new(atom_spans.len(), policy, limit, 0);
    let offset = 0;
    while offset < candidates.len() {
        let start_byte = candidates[offset].span.token.start;
        let end_offset = offset
            + candidates[offset..]
                .partition_point(|candidate| candidate.span.token.start == start_byte);
        let start = metrics.advance_to(start_byte);
        selector.advance_to(start);
        if selector.settle() {
            return selector.finish();
        }
        for candidate in candidates.drain(offset..end_offset) {
            let sequence = next_sequence[candidate.atom_index];
            next_sequence[candidate.atom_index] += 1;
            let Some(token) = text.get(candidate.span.token.clone()) else {
                continue;
            };
            let end = start.advance(token);
            selector.push(
                IndexedCandidate {
                    atom_index: candidate.atom_index,
                    sequence,
                    span: candidate.span,
                },
                end,
            );
        }
        selector.finish_group(text, start_byte, start);
        if selector.settle() {
            return selector.finish();
        }
    }
    selector.clear_active();
    selector.settle();
    selector.finish()
}
