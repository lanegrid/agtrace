//! Context-window wiring for the workspace fold.
//!
//! Each agent folds its own events into a [`ContextEvidence`] (`crate::context`);
//! whenever that evidence changes the fold asks a [`WindowResolver`] for the window.
//! The production resolver is [`CatalogResolver`], i.e. `context::resolve` against an
//! injected [`ModelCatalog`]; tests may pass a closure or [`NoWindow`].

use agtrace_types::{AgentRef, ModelCatalog};

pub use crate::context::ContextEvidence;
pub use agtrace_types::ContextWindow;

/// Resolves an agent's context window from its accumulated evidence.
pub trait WindowResolver {
    fn resolve(&self, agent: &AgentRef, evidence: &ContextEvidence) -> Option<ContextWindow>;
}

/// `context::resolve(agent.provider, evidence, catalog)`.
#[derive(Clone, Copy)]
pub struct CatalogResolver<'a>(pub &'a dyn ModelCatalog);

impl WindowResolver for CatalogResolver<'_> {
    fn resolve(&self, agent: &AgentRef, evidence: &ContextEvidence) -> Option<ContextWindow> {
        crate::context::resolve(agent.provider, evidence, self.0)
    }
}

/// Resolver that never knows the window.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoWindow;

impl WindowResolver for NoWindow {
    fn resolve(&self, _agent: &AgentRef, _evidence: &ContextEvidence) -> Option<ContextWindow> {
        None
    }
}

impl<F> WindowResolver for F
where
    F: Fn(&AgentRef, &ContextEvidence) -> Option<ContextWindow>,
{
    fn resolve(&self, agent: &AgentRef, evidence: &ContextEvidence) -> Option<ContextWindow> {
        self(agent, evidence)
    }
}
