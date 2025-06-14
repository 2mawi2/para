#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_no_real_ide_launched_in_tests() {
        // This test ensures that test configurations always use mock IDE commands
        let temp_dir = TempDir::new().unwrap();
        let test_config_path = temp_dir.path().join("test-config.json");

        // Create a config through our test utilities
        let config = crate::test_utils::test_helpers::create_test_config();

        // Verify IDE commands are mocked
        assert_eq!(
            config.ide.command, "echo",
            "IDE command must be mocked in tests"
        );
        assert_eq!(
            config.ide.wrapper.command, "echo",
            "Wrapper command must be mocked in tests"
        );

        // Write and reload to ensure it persists
        let config_json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&test_config_path, config_json).unwrap();

        // Load config as the system would
        let loaded_config =
            crate::config::ConfigManager::load_from_file(&test_config_path).unwrap();

        // Verify loaded config is still mocked
        assert_eq!(
            loaded_config.ide.command, "echo",
            "Loaded IDE command must remain mocked"
        );
        assert_ne!(
            loaded_config.ide.command, "cursor",
            "Real IDE command must not be present"
        );
        assert_ne!(
            loaded_config.ide.command, "claude",
            "Real IDE command must not be present"
        );
        assert_ne!(
            loaded_config.ide.command, "code",
            "Real IDE command must not be present"
        );
    }

    #[test]
    fn test_environment_guard_creates_test_config() {
        use crate::test_utils::test_helpers::TestEnvironmentGuard;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Test that guard creates a test config file
        let guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config_path = guard.config_path();

        // Verify config file exists
        assert!(config_path.exists(), "Test config file should be created");

        // Verify the config uses mock IDE
        let config = crate::config::ConfigManager::load_from_file(config_path)
            .expect("Should be able to load test config");
        assert_eq!(
            config.ide.command, "echo",
            "Test config should use mock IDE"
        );
        assert_eq!(
            config.ide.name, "test-ide",
            "Test config should use test IDE name"
        );
    }

    #[test]
    fn test_config_isolation_uses_test_config_explicitly() {
        use crate::config::ConfigManager;
        use crate::test_utils::test_helpers::TestEnvironmentGuard;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Create a guard to set up test environment
        let guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Tests should explicitly load from the test config path
        let config = ConfigManager::load_from_file(guard.config_path()).unwrap();

        // Verify we get the isolated test config
        assert_eq!(
            config.ide.command, "echo",
            "Test config should use mock IDE command"
        );
        assert_eq!(
            config.ide.name, "test-ide",
            "Test config should use test IDE name"
        );

        // Verify that load_or_create() would point to a different config
        // (we can't actually load it as it might have an invalid IDE command)
        let default_config_path = ConfigManager::get_config_path().unwrap();
        assert_ne!(
            guard.config_path().to_string_lossy(),
            default_config_path,
            "Test config path should be different from default user config path"
        );
    }
}
