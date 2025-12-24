use anyhow::Result;
use serde::Serialize;

use crate::presentation::view_models::{CommandResultViewModel, CreateView};

pub trait Renderer {
    fn render<T>(&self, result: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + CreateView + Send + Sync;
}
