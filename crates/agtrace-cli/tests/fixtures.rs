use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestFixture {
    _temp_dir: TempDir,
    data_dir: PathBuf,
    log_root: PathBuf,
}

impl TestFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = temp_dir.path().join(".agtrace");
        let log_root = temp_dir.path().join(".claude");

        fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        fs::create_dir_all(&log_root).expect("Failed to create log dir");

        Self {
            _temp_dir: temp_dir,
            data_dir,
            log_root,
        }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn log_root(&self) -> &PathBuf {
        &self.log_root
    }

    pub fn copy_sample_file(&self, sample_name: &str, dest_name: &str) -> anyhow::Result<()> {
        let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        let source = samples_dir.join(sample_name);
        let dest = self.log_root.join(dest_name);

        fs::copy(source, dest)?;
        Ok(())
    }

    pub fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("agtrace").expect("Failed to find agtrace binary");
        cmd.arg("--data-dir")
            .arg(self.data_dir())
            .arg("--format")
            .arg("plain");
        cmd
    }

    pub fn setup_provider(&self, provider_name: &str) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("provider")
            .arg("set")
            .arg(provider_name)
            .arg("--log-root")
            .arg(self.log_root())
            .arg("--enable")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "provider set failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub fn index_update(&self) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("index")
            .arg("update")
            .arg("--all-projects")
            .arg("--verbose")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "index update failed: {}\nstdout: {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }
}
