/// Display formatting options for rendering
///
/// This is a cross-cutting concern used by:
/// - view_models (as part of ViewModel context)
/// - formatters (for formatting primitives)
/// - renderers (passed through trait methods)
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub enable_color: bool,
    pub relative_time: bool,
    pub truncate_text: Option<usize>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        }
    }
}
