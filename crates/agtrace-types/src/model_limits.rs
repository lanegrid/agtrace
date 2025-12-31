#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelSpec {
    pub max_tokens: u64,
    pub compaction_buffer_pct: f64,
}

pub trait ModelLimitResolver {
    fn resolve_model_limit(&self, model: &str) -> Option<ModelSpec>;
}
