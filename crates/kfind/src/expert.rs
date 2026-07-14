//! Opt-in access to kfind's mutable query-planning internals.
//!
//! This module is not part of the 1.x stable facade contract. Prefer
//! [`crate::ResourceBundle`] and [`crate::Engine::with_resources`] unless a
//! caller must assemble lexicons or inspect the compiled plan directly.

use kfind_data::DataError;

use crate::{Engine, Matcher};

pub use kfind_query::{Lexicons, QueryPlan};

/// Opt-in constructors for caller-assembled lexicons.
pub trait EngineExt: Sized {
    /// Creates an engine from caller-configured lexicons.
    #[must_use]
    fn from_lexicons(lexicons: Lexicons) -> Self;

    /// Creates an engine from caller-configured lexicons and a component resource.
    fn from_lexicons_with_component(
        lexicons: Lexicons,
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<Self, DataError>;
}

impl EngineExt for Engine {
    fn from_lexicons(lexicons: Lexicons) -> Self {
        Self::from_lexicons(lexicons)
    }

    fn from_lexicons_with_component(
        lexicons: Lexicons,
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<Self, DataError> {
        Self::from_lexicons_with_component(lexicons, component_resource)
    }
}

/// Opt-in access to a matcher's compiled query plan.
pub trait MatcherExt {
    /// Returns the mutable expert query-plan representation.
    #[must_use]
    fn plan(&self) -> &QueryPlan;
}

impl MatcherExt for Matcher {
    fn plan(&self) -> &QueryPlan {
        self.inner.plan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompileOptions;

    #[test]
    fn expert_traits_require_explicit_opt_in() {
        let lexicons = Lexicons::embedded().unwrap();
        let engine = Engine::from_lexicons(lexicons);
        let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();

        assert_eq!(matcher.plan().atoms.len(), 1);
    }
}
