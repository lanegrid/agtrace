use agtrace_types::{ToolKind, ToolOrigin};

/// Tool specification with origin and semantic kind
pub struct ToolSpec {
    pub name: &'static str,
    pub origin: ToolOrigin,
    pub kind: ToolKind,
}

impl ToolSpec {
    pub const fn new(name: &'static str, origin: ToolOrigin, kind: ToolKind) -> Self {
        Self { name, origin, kind }
    }
}
