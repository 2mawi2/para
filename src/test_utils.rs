pub mod test_helpers {
    use crate::config::Config;
    use crate::core::git::GitService;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    pub fn create_test_config() -> Config {
        crate::config::defaults::default_config()
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

    pub fn setup_isolated_test_environment(temp_dir: &TempDir) -> (PathBuf, String) {
        // Create a test config that points to our temp state dir
        let config_dir = temp_dir.path().join(".config").join("para");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.json");

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        let config_json =
            serde_json::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_file, config_json).expect("Failed to write config file");

        // Return the config directory and original HOME for restoration
        let original_home = std::env::var("HOME").unwrap_or_default();
        std::env::set_var("HOME", temp_dir.path());

        (config_dir, original_home)
    }

    pub fn restore_environment(original_dir: PathBuf, original_home: String) {
        // Try to restore directory, but don't panic if it fails
        if let Err(_e) = std::env::set_current_dir(&original_dir) {
            // If we can't restore to the original directory, try to go to a safe fallback
            let _ = std::env::set_current_dir("/tmp");
        }

        if !original_home.is_empty() {
            std::env::set_var("HOME", original_home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    pub struct TestEnvironmentGuard {
        original_dir: PathBuf,
        original_home: String,
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

            std::env::set_current_dir(git_temp.path())?;

            let (_config_dir, original_home) = setup_isolated_test_environment(temp_dir);

            Ok(TestEnvironmentGuard {
                original_dir,
                original_home,
            })
        }
    }

    impl Drop for TestEnvironmentGuard {
        fn drop(&mut self) {
            restore_environment(self.original_dir.clone(), self.original_home.clone());
        }
    }
}
