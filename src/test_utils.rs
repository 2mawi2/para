#[cfg(test)]
pub mod mock_platform;

#[cfg(test)]
pub mod test_safety;

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
        config
    }

    pub fn setup_test_repo() -> (TempDir, GitService) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

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

    pub fn restore_environment(original_dir: PathBuf) {
        // Try to restore directory, but don't panic if it fails
        if let Err(_e) = std::env::set_current_dir(&original_dir) {
            // If we can't restore to the original directory, try to go to a safe fallback
            let _ = std::env::set_current_dir("/tmp");
        }
    }

    pub struct TestEnvironmentGuard {
        original_dir: PathBuf,
        test_dir: PathBuf,
        test_config_path: PathBuf,
        original_para_config_path: Option<String>,
    }

    impl TestEnvironmentGuard {
        pub fn new(git_temp: &TempDir, temp_dir: &TempDir) -> Result<Self, std::io::Error> {
            // Try to get the original directory, but if it fails, use a fallback
            let original_dir = std::env::current_dir().unwrap_or_else(|_| {
                // If current dir is invalid, try to use the git_temp parent as fallback
                git_temp
                    .path()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("/tmp"))
                    .to_path_buf()
            });

            // Store original PARA_CONFIG_PATH if it exists
            let original_para_config_path = std::env::var("PARA_CONFIG_PATH").ok();

            std::env::set_current_dir(git_temp.path())?;

            let _config_dir = setup_isolated_test_environment(temp_dir);

            // Create test config file
            let test_config_path = temp_dir.path().join("test-config.json");
            let test_config = create_test_config();
            let config_json = serde_json::to_string_pretty(&test_config)
                .expect("Failed to serialize test config");
            fs::write(&test_config_path, config_json)?;

            // Set PARA_CONFIG_PATH to our test config to override default config resolution
            std::env::set_var("PARA_CONFIG_PATH", &test_config_path);

            Ok(TestEnvironmentGuard {
                original_dir,
                test_dir: git_temp.path().to_path_buf(),
                test_config_path,
                original_para_config_path,
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

            // Restore original PARA_CONFIG_PATH
            match &self.original_para_config_path {
                Some(path) => std::env::set_var("PARA_CONFIG_PATH", path),
                None => std::env::remove_var("PARA_CONFIG_PATH"),
            }

            restore_environment(self.original_dir.clone());
        }
    }
}
