#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_no_real_ide_launched_in_tests() {
        // This test ensures that test configurations always use mock IDE commands
        let temp_dir = TempDir::new().unwrap();
        let test_config_path = temp_dir.path().join("test-config.json");

        // Set environment to use test config
        std::env::set_var(
            "PARA_CONFIG_PATH",
            test_config_path.to_string_lossy().as_ref(),
        );

        // Create a config through our test utilities
        let config = crate::test_utils::test_helpers::create_test_config_with_mock_ide();

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

        // Clean up
        std::env::remove_var("PARA_CONFIG_PATH");
    }

    #[test]
    fn test_environment_guard_isolates_config() {
        use crate::test_utils::test_helpers::TestEnvironmentGuard;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Test scenario 1: No initial config path
        std::env::remove_var("PARA_CONFIG_PATH");
        {
            let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

            // During guard - config path is set to test location
            let config_path = std::env::var("PARA_CONFIG_PATH").unwrap();
            assert!(config_path.contains("test-config.json"));

            // Verify the config at that path uses mock IDE
            let config = crate::config::ConfigManager::load_from_file(&std::path::PathBuf::from(
                &config_path,
            ))
            .unwrap();
            assert_eq!(config.ide.command, "echo");
        }
        // After guard - should be removed if it wasn't set before
        assert!(std::env::var("PARA_CONFIG_PATH").is_err());

        // Test scenario 2: Existing config path
        let test_value = "/tmp/existing-config.json";
        std::env::set_var("PARA_CONFIG_PATH", test_value);
        {
            let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

            // During guard - config path is overridden
            let config_path = std::env::var("PARA_CONFIG_PATH").unwrap();
            assert!(config_path.contains("test-config.json"));
            assert_ne!(config_path, test_value);
        }
        // After guard - should be restored to original value
        assert_eq!(std::env::var("PARA_CONFIG_PATH").unwrap(), test_value);

        // Clean up
        std::env::remove_var("PARA_CONFIG_PATH");
    }
}
