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

        #[test]
        fn test_parse_launch_file_contents_wrapper_mode_cursor() {
            let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=cursor\nLAUNCH_IDE=claude";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "cursor");
        }

        #[test]
        fn test_parse_launch_file_contents_wrapper_mode_code() {
            let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=code\nLAUNCH_IDE=claude";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "code");
        }

        #[test]
        fn test_parse_launch_file_contents_wrapper_mode_default() {
            let contents = "LAUNCH_METHOD=wrapper\nLAUNCH_IDE=claude";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_parse_launch_file_contents_launch_ide() {
            let contents = "LAUNCH_IDE=cursor\nSOME_OTHER=value";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "cursor");
        }

        #[test]
        fn test_parse_launch_file_contents_empty() {
            let contents = "";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_parse_launch_file_contents_no_ide_info() {
            let contents = "SOME_KEY=value\nANOTHER_KEY=value2";
            let result = MacOSPlatform::parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_format_search_fragment_cursor_with_timestamp() {
            let session_id = "my-feature-20250615-123456";
            let result = MacOSPlatform::format_search_fragment(session_id, "cursor");
            assert_eq!(result, "my-feature");
        }

        #[test]
        fn test_format_search_fragment_cursor_docker_style() {
            let session_id = "eager_phoenix";
            let result = MacOSPlatform::format_search_fragment(session_id, "cursor");
            assert_eq!(result, "eager_phoenix");
        }

        #[test]
        fn test_format_search_fragment_vscode_preserves_full_name() {
            let session_id = "my-feature-20250615-123456";
            let result = MacOSPlatform::format_search_fragment(session_id, "code");
            assert_eq!(result, "my-feature-20250615-123456");
        }

        #[test]
        fn test_format_search_fragment_other_ide_preserves_full_name() {
            let session_id = "my-feature-20250615-123456";
            let result = MacOSPlatform::format_search_fragment(session_id, "other");
            assert_eq!(result, "my-feature-20250615-123456");
        }

        #[test]
        fn test_generate_applescript_contains_expected_elements() {
            let script = MacOSPlatform::generate_applescript("Cursor", "my-feature");

            // Verify key elements are present in the generated script
            assert!(script.contains("set appName to \"Cursor\""));
            assert!(script.contains("set windowTitleFragment to \"my-feature\""));
            assert!(script.contains("tell application \"System Events\""));
            assert!(script.contains("every window whose name contains windowTitleFragment"));
            assert!(script.contains("perform action \"AXRaise\" of targetWindow"));
            assert!(script.contains("click (button 1 of targetWindow)"));
        }

        #[test]
        fn test_generate_applescript_different_app_names() {
            let script_cursor = MacOSPlatform::generate_applescript("Cursor", "session");
            let script_code = MacOSPlatform::generate_applescript("Code", "session");

            assert!(script_cursor.contains("set appName to \"Cursor\""));
            assert!(script_code.contains("set appName to \"Code\""));
        }

        #[test]
        fn test_generate_applescript_different_search_fragments() {
            let script1 = MacOSPlatform::generate_applescript("Cursor", "feature-branch");
            let script2 = MacOSPlatform::generate_applescript("Cursor", "eager_phoenix");

            assert!(script1.contains("set windowTitleFragment to \"feature-branch\""));
            assert!(script2.contains("set windowTitleFragment to \"eager_phoenix\""));
        }
    }
}
