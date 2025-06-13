#[cfg(test)]
mod platform_tests {
    use crate::platform::get_platform_manager;

    #[test]
    fn test_platform_manager_creation() {
        // Just test that we can create a platform manager
        // Don't actually call close_ide_window as it's forbidden in tests
        let _platform = get_platform_manager();
    }

    #[test]
    #[should_panic(expected = "CRITICAL: close_ide_window called from test environment!")]
    fn test_close_ide_window_panics_in_tests() {
        // This test verifies that close_ide_window properly panics when called from tests
        let platform = get_platform_manager();
        let _ = platform.close_ide_window("test-session", "cursor");
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use crate::platform::{macos::MacOSPlatform, PlatformManager};

        #[test]
        #[should_panic(expected = "CRITICAL: close_ide_window called from test environment!")]
        fn test_macos_close_ide_window_panics_in_tests() {
            // Verify that MacOSPlatform specifically panics in test environment
            let platform = MacOSPlatform;
            let _ = platform.close_ide_window("test-session", "cursor");
        }
    }
}
