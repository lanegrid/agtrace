use agtrace_index::Database;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub enum ScanDecision {
    ShouldScan,
    Skip { reason: SkipReason },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    RecentlyScanned { elapsed: Duration },
}

pub struct InitService;

impl InitService {
    pub fn should_rescan(
        db: &Database,
        project_hash: &str,
        force_refresh: bool,
    ) -> Result<ScanDecision> {
        if force_refresh {
            return Ok(ScanDecision::ShouldScan);
        }

        if let Ok(Some(project)) = db.get_project(project_hash) {
            if let Some(last_scanned) = &project.last_scanned_at {
                if let Ok(last_time) = DateTime::parse_from_rfc3339(last_scanned) {
                    let elapsed = Utc::now().signed_duration_since(last_time.with_timezone(&Utc));
                    if elapsed < Duration::minutes(5) {
                        return Ok(ScanDecision::Skip {
                            reason: SkipReason::RecentlyScanned { elapsed },
                        });
                    }
                }
            }
        }

        Ok(ScanDecision::ShouldScan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_index::ProjectRecord;
    use tempfile::TempDir;

    #[test]
    fn test_force_refresh_always_scans() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path)?;

        let decision = InitService::should_rescan(&db, "test_hash", true)?;
        assert!(matches!(decision, ScanDecision::ShouldScan));

        Ok(())
    }

    #[test]
    fn test_nonexistent_project_scans() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path)?;

        let decision = InitService::should_rescan(&db, "nonexistent", false)?;
        assert!(matches!(decision, ScanDecision::ShouldScan));

        Ok(())
    }

    #[test]
    fn test_recently_scanned_skips() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path)?;

        let project_hash = "test_hash";
        let now = Utc::now();
        let recent_time = now - Duration::minutes(2);

        db.insert_or_update_project(&ProjectRecord {
            hash: project_hash.to_string(),
            root_path: Some("/test/path".to_string()),
            last_scanned_at: Some(recent_time.to_rfc3339()),
        })?;

        let decision = InitService::should_rescan(&db, project_hash, false)?;
        assert!(matches!(
            decision,
            ScanDecision::Skip {
                reason: SkipReason::RecentlyScanned { .. }
            }
        ));

        Ok(())
    }

    #[test]
    fn test_old_scan_rescans() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path)?;

        let project_hash = "test_hash";
        let now = Utc::now();
        let old_time = now - Duration::minutes(10);

        db.insert_or_update_project(&ProjectRecord {
            hash: project_hash.to_string(),
            root_path: Some("/test/path".to_string()),
            last_scanned_at: Some(old_time.to_rfc3339()),
        })?;

        let decision = InitService::should_rescan(&db, project_hash, false)?;
        assert!(matches!(decision, ScanDecision::ShouldScan));

        Ok(())
    }
}
