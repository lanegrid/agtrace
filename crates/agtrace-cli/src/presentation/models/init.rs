use chrono::Duration;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InitDisplay {
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub step1: Step1Result,
    pub step2: Step2Result,
    pub step3: Step3Result,
    pub step4: Step4Result,
}

#[derive(Debug, Clone)]
pub enum Step1Result {
    DetectedProviders {
        providers: HashMap<String, PathBuf>,
        config_saved: bool,
    },
    LoadedConfig {
        config_path: PathBuf,
    },
    NoProvidersDetected,
}

#[derive(Debug, Clone)]
pub struct Step2Result {
    pub db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum Step3Result {
    Scanned {
        success: bool,
        error: Option<String>,
    },
    Skipped {
        reason: SkipReason,
    },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    RecentlyScanned { elapsed: Duration },
}

#[derive(Debug, Clone)]
pub struct Step4Result {
    pub session_count: usize,
    pub first_session_id: Option<String>,
    pub all_projects: bool,
}

impl InitDisplay {
    pub fn new(config_path: PathBuf, db_path: PathBuf) -> Self {
        let db_path_clone = db_path.clone();
        Self {
            config_path,
            db_path,
            step1: Step1Result::NoProvidersDetected,
            step2: Step2Result {
                db_path: db_path_clone,
            },
            step3: Step3Result::Skipped {
                reason: SkipReason::RecentlyScanned {
                    elapsed: Duration::zero(),
                },
            },
            step4: Step4Result {
                session_count: 0,
                first_session_id: None,
                all_projects: false,
            },
        }
    }

    pub fn with_step1(mut self, result: Step1Result) -> Self {
        self.step1 = result;
        self
    }

    pub fn with_step2(mut self, result: Step2Result) -> Self {
        self.step2 = result;
        self
    }

    pub fn with_step3(mut self, result: Step3Result) -> Self {
        self.step3 = result;
        self
    }

    pub fn with_step4(mut self, result: Step4Result) -> Self {
        self.step4 = result;
        self
    }
}
