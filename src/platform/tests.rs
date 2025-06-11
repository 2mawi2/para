#[cfg(test)]
mod platform_tests {
    use crate::platform::get_platform_manager;

    #[test]
    fn test_platform_manager_creation() {
        let platform = get_platform_manager();

        // Test that we can create a platform manager
        // This should work on all platforms
        assert!(platform.close_ide_window("test-session", "cursor").is_ok());
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use crate::platform::{macos::MacOSPlatform, PlatformManager};

        #[test]
        fn test_close_ide_window() {
            let platform = MacOSPlatform;

            // Test closing IDE windows - this should not fail even if no IDE is running
            let result = platform.close_ide_window("test-session", "cursor");
            assert!(result.is_ok());

            let result = platform.close_ide_window("test-session", "code");
            assert!(result.is_ok());

            // Test unsupported IDE
            let result = platform.close_ide_window("test-session", "unsupported");
            assert!(result.is_ok());
        }
    }
}
