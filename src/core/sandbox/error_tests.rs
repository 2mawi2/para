#[cfg(test)]
mod error_path_tests {
    use super::super::*;
    use crate::config::Config;
    use crate::core::sandbox::{config::SandboxResolver, SandboxConfig};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_migration_handles_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");

        // Write invalid JSON
        fs::write(&config_path, "{ invalid json }").unwrap();

        // Migration should fail gracefully
        let result = crate::config::migration::migrate_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_handles_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.json");

        // Should succeed (no-op) when file doesn't exist
        let result = crate::config::migration::migrate_config(&config_path);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_profile_extraction_with_empty_name() {
        let result = profiles::extract_profile("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_profile_extraction_cleanup_on_error() {
        // Try to extract invalid profile
        let result = profiles::extract_profile("../../etc/passwd");
        assert!(result.is_err());

        // Verify no malicious files were created
        let temp_dir = std::env::temp_dir().join("para-sandbox-profiles");
        if temp_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&temp_dir)
                .unwrap()
                .filter_map(Result::ok)
                .collect();

            // Should not contain any path traversal attempts
            for entry in entries {
                let name = entry.file_name();
                assert!(!name.to_string_lossy().contains(".."));
                assert!(!name.to_string_lossy().contains("etc"));
                assert!(!name.to_string_lossy().contains("passwd"));
            }
        }
    }

    #[test]
    fn test_sandbox_resolver_with_invalid_profile() {
        // Test with invalid profile in config
        let config = Config {
            sandbox: Some(SandboxConfig {
                enabled: true,
                profile: "../../../../etc/passwd".to_string(),
            }),
            ..crate::config::defaults::default_config()
        };

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);

        // Should fall back to default profile
        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard");
    }

    #[test]
    fn test_sandbox_launcher_empty_profile() {
        use launcher::wrap_with_sandbox;

        let temp_dir = TempDir::new().unwrap();
        let result = wrap_with_sandbox("echo test", temp_dir.path(), "");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_sandbox_launcher_invalid_profile() {
        use launcher::wrap_with_sandbox;

        let temp_dir = TempDir::new().unwrap();
        let result = wrap_with_sandbox("echo test", temp_dir.path(), "nonexistent-profile");

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid or unknown") || err_msg.contains("Unknown sandbox profile") || err_msg.contains("Failed to extract sandbox profile"),
            "Expected error message to contain 'Invalid or unknown', 'Unknown sandbox profile', or 'Failed to extract sandbox profile', got: {err_msg}"
        );
    }

    #[test]
    fn test_is_sandbox_available_error_handling() {
        // This test verifies is_sandbox_available doesn't panic
        let _available = launcher::is_sandbox_available();
        // Should complete without panicking
    }

    #[test]
    fn test_cleanup_handles_permission_errors() {
        // Create a directory that cleanup can process
        let temp_dir = std::env::temp_dir().join("para-sandbox-profiles-test");
        fs::create_dir_all(&temp_dir).unwrap();

        // Create a file
        let test_file = temp_dir.join("test.sb");
        fs::write(&test_file, "content").unwrap();

        // Make directory read-only on Unix to simulate permission error
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&temp_dir, perms).unwrap();
        }

        // Cleanup should handle errors gracefully
        cleanup::cleanup_old_profiles().ok();

        // Restore permissions and clean up
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_dir, perms).unwrap();
        }

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_config_migration_preserves_data_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a valid config
        let original_config = r#"{
            "ide": {"name": "test", "command": "echo"},
            "important_data": "should_be_preserved"
        }"#;
        fs::write(&config_path, original_config).unwrap();

        // Make file read-only to cause write error during migration
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&config_path, perms).ok();
        }

        // Attempt migration (may fail on write)
        let _ = crate::config::migration::migrate_config(&config_path);

        // Restore permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&config_path, perms).ok();
        }

        // Verify original data is still intact
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("important_data"));
    }

    #[test]
    fn test_profile_validation_edge_cases() {
        // Test valid profiles through from_name
        assert!(profiles::SandboxProfile::from_name("permissive-open").is_some());
        assert!(profiles::SandboxProfile::from_name("test-123").is_none()); // Valid format but unknown

        // Test maximum length profile name (valid format but unknown)
        let long_name = "a".repeat(50);
        assert!(profiles::SandboxProfile::from_name(&long_name).is_none());

        // Test just over maximum length
        let too_long = "a".repeat(51);
        assert!(profiles::SandboxProfile::from_name(&too_long).is_none());

        // Test Unicode characters (should be rejected)
        assert!(profiles::SandboxProfile::from_name("test-🦀").is_none());
        assert!(profiles::SandboxProfile::from_name("тест").is_none());

        // Test various special characters
        let special_chars = vec![
            "test!profile",
            "test@profile",
            "test#profile",
            "test$profile",
            "test%profile",
            "test^profile",
            "test&profile",
            "test*profile",
            "test(profile",
            "test)profile",
            "test=profile",
            "test+profile",
            "test[profile",
            "test]profile",
            "test{profile",
            "test}profile",
            "test|profile",
            "test\\profile",
            "test:profile",
            "test;profile",
            "test'profile",
            "test\"profile",
            "test<profile",
            "test>profile",
            "test,profile",
            "test.profile",
            "test?profile",
            "test~profile",
            "test`profile",
        ];

        for name in special_chars {
            assert!(
                profiles::SandboxProfile::from_name(name).is_none(),
                "Should reject profile name with special char: {name}"
            );
        }
    }
}
