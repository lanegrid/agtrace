use anyhow::Result;
use serde::Serialize;

use crate::presentation::v2::view_models::CommandResultViewModel;

pub trait ConsolePresentable {
    fn render_console(&self);
}

pub trait Renderer {
    fn render<T>(&self, result: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + ConsolePresentable + Send + Sync;
}
