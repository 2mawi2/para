#[cfg(test)]
pub mod mock_platform;

#[cfg(test)]
pub mod test_safety;

#[cfg(test)]
pub mod test_helpers {
    use crate::config::Config;
    use crate::core::git::GitService;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    pub fn create_test_config() -> Config {
        let mut config = crate::config::defaults::default_config();
        // Always use mock IDE commands in tests
        config.ide.name = "test-ide".to_string();
        config.ide.command = "echo".to_string();
        config.ide.wrapper.command = "echo".to_string();
        // Sandbox is None by default in default_config
        config
    }

    pub fn create_test_config_with_dir(temp_dir: &TempDir) -> Config {
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees".to_string();
        config.git.branch_prefix = "test".to_string();
        config
    }

    pub fn setup_test_repo() -> (TempDir, GitService) {
        setup_test_repo_fast()
    }

    /// Optimized git repository setup that reduces process spawning overhead
    pub fn setup_test_repo_fast() -> (TempDir, GitService) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        // Create README before git init to reduce operations
        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        // Initialize git repo with all config in one command
        Command::new("sh")
            .current_dir(repo_path)
            .arg("-c")
            .arg(
                r#"
                git init --initial-branch=main &&
                git config user.name "Test User" &&
                git config user.email "test@example.com" &&
                git add . &&
                git commit -m "Initial commit"
            "#,
            )
            .status()
            .expect("Failed to setup git repo");

        let service = GitService::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, service)
    }

    pub fn setup_isolated_test_environment(temp_dir: &TempDir) -> PathBuf {
        // Create a test config that points to our temp state dir
        let config_dir = temp_dir.path().join(".config").join("para");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.json");

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        let config_json =
            serde_json::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_file, config_json).expect("Failed to write config file");

        config_dir
    }

    pub struct TestEnvironmentGuard {
        test_dir: PathBuf,
        test_config_path: PathBuf,
    }

    impl TestEnvironmentGuard {
        pub fn new(git_temp: &TempDir, temp_dir: &TempDir) -> Result<Self, std::io::Error> {
            // Don't change the current directory - this causes race conditions in parallel tests
            // Tests should use absolute paths instead

            let _config_dir = setup_isolated_test_environment(temp_dir);

            // Create test config file
            let test_config_path = temp_dir.path().join("test-config.json");
            let test_config = create_test_config();
            let config_json = serde_json::to_string_pretty(&test_config)
                .expect("Failed to serialize test config");
            fs::write(&test_config_path, config_json)?;

            Ok(TestEnvironmentGuard {
                test_dir: git_temp.path().to_path_buf(),
                test_config_path,
            })
        }

        pub fn config_path(&self) -> &Path {
            &self.test_config_path
        }
    }

    impl Drop for TestEnvironmentGuard {
        fn drop(&mut self) {
            // Clean up test artifacts that could interfere with git operations
            // Only clean up in the test directory we created, not wherever we might be now
            if let Ok(entries) = fs::read_dir(&self.test_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join(".git").exists() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                        // Check if this looks like a test artifact
                        // More restrictive patterns to avoid false positives
                        if name.starts_with("test-")
                            || name == "temp"
                            || name.ends_with("-test")
                            || name.contains("-validation")
                            || name.starts_with("test_")
                        {
                            eprintln!("TestEnvironmentGuard: Cleaning up test artifact with nested git: {}", path.display());
                            if let Err(e) = fs::remove_dir_all(&path) {
                                eprintln!(
                                    "TestEnvironmentGuard: Failed to clean up {}: {}",
                                    path.display(),
                                    e
                                );
                            } else {
                                eprintln!(
                                    "TestEnvironmentGuard: Successfully cleaned up {}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
            }

            // Don't restore directory in parallel tests - it causes race conditions
            // restore_environment(self.original_dir.clone());
        }
    }
}
