use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;

use crate::presentation::v2::view_models::CommandResultViewModel;

pub trait Renderer {
    fn render<T>(&self, result: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + Display + Send + Sync;
}
