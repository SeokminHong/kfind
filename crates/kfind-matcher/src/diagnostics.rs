use std::cell::Cell;

/// Caller-owned diagnostics accumulated across searches in one scope.
///
/// A false flag does not guarantee exhaustive search: early exits, unsupported
/// morphology, and inputs outside the search scope are not diagnosed here.
#[derive(Debug, Default)]
pub struct SearchDiagnostics {
    structural_verification_incomplete: Cell<bool>,
}

impl SearchDiagnostics {
    /// Whether a structural candidate could not be evaluated in this scope.
    #[must_use]
    pub fn structural_verification_incomplete(&self) -> bool {
        self.structural_verification_incomplete.get()
    }

    pub(crate) fn record_unavailable(&self) {
        self.structural_verification_incomplete.set(true);
    }
}
