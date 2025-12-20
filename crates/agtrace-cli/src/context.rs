use agtrace_runtime::AgTrace;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

pub struct ExecutionContext {
    data_dir: PathBuf,
    workspace: OnceCell<AgTrace>,
    pub project_root: Option<PathBuf>,
    pub all_projects: bool,
}

impl ExecutionContext {
    pub fn new(
        data_dir: PathBuf,
        project_root: Option<String>,
        all_projects: bool,
    ) -> Result<Self> {
        let project_root = project_root
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Ok(Self {
            data_dir,
            project_root,
            all_projects,
            workspace: OnceCell::new(),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn workspace(&self) -> Result<&AgTrace> {
        self.workspace.get_or_try_init(|| {
            AgTrace::open(self.data_dir.clone())
                .context("Failed to open agtrace workspace. Have you run 'agtrace init'?")
        })
    }
}

