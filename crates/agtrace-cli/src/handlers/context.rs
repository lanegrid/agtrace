use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::view_models::{CommandResultViewModel, CreateView};
use crate::presentation::{ConsoleRenderer, Renderer, ViewMode};
use anyhow::Result;
use serde::Serialize;

/// Context for handler execution with consistent presentation utilities
pub struct HandlerContext {
    pub format: OutputFormat,
    pub view_mode: ViewMode,
}

impl HandlerContext {
    pub fn new(format: OutputFormat, view_mode: &ViewModeArgs) -> Self {
        Self {
            format,
            view_mode: view_mode.resolve(),
        }
    }

    /// Render a view model using the configured format and view mode
    pub fn render<T>(&self, view_model: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + CreateView + Send + Sync,
    {
        let renderer = ConsoleRenderer::new(self.format.into(), self.view_mode);
        renderer.render(view_model)
    }
}
