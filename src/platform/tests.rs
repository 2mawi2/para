#[cfg(test)]
pub mod platform_tests {
    use crate::platform::get_platform_manager;

    /// Test utility function for parsing launch file contents
    /// This is isolated to test code to avoid polluting production code with test-only logic
    pub(crate) fn parse_launch_file_contents(contents: &str, default_ide: &str) -> String {
        if contents.contains("LAUNCH_METHOD=wrapper") {
            // For wrapper mode, Claude Code runs inside Cursor/VS Code
            if contents.contains("WRAPPER_IDE=cursor") {
                "cursor".to_string()
            } else if contents.contains("WRAPPER_IDE=code") {
                "code".to_string()
            } else {
                // Default to configured IDE wrapper name
                default_ide.to_string()
            }
        } else if let Some(line) = contents.lines().find(|l| l.starts_with("LAUNCH_IDE=")) {
            line.split('=').nth(1).unwrap_or(default_ide).to_string()
        } else {
            default_ide.to_string()
        }
    }

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
        use crate::platform::{
            macos::{IdeHandler, MacOSPlatform},
            tests::platform_tests::parse_launch_file_contents,
            PlatformManager,
        };

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
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "cursor");
        }

        #[test]
        fn test_parse_launch_file_contents_wrapper_mode_code() {
            let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=code\nLAUNCH_IDE=claude";
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "code");
        }

        #[test]
        fn test_parse_launch_file_contents_wrapper_mode_default() {
            let contents = "LAUNCH_METHOD=wrapper\nLAUNCH_IDE=claude";
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_parse_launch_file_contents_launch_ide() {
            let contents = "LAUNCH_IDE=cursor\nSOME_OTHER=value";
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "cursor");
        }

        #[test]
        fn test_parse_launch_file_contents_empty() {
            let contents = "";
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_parse_launch_file_contents_no_ide_info() {
            let contents = "SOME_KEY=value\nANOTHER_KEY=value2";
            let result = parse_launch_file_contents(contents, "default");
            assert_eq!(result, "default");
        }

        #[test]
        fn test_format_search_fragment_cursor_with_timestamp() {
            let platform = MacOSPlatform;
            let session_info = platform
                .parse_session_info("my-feature-20250615-123456")
                .unwrap();
            let handler = crate::platform::macos::CursorHandler;
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_format_search_fragment_cursor_docker_style() {
            let platform = MacOSPlatform;
            let session_info = platform.parse_session_info("eager_phoenix").unwrap();
            let handler = crate::platform::macos::CursorHandler;
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"eager_phoenix\""));
        }

        #[test]
        fn test_format_search_fragment_vscode_preserves_full_name() {
            let platform = MacOSPlatform;
            let session_info = platform
                .parse_session_info("my-feature-20250615-123456")
                .unwrap();
            let handler = crate::platform::macos::VSCodeHandler;
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_format_search_fragment_other_ide_preserves_full_name() {
            let platform = MacOSPlatform;
            let session_info = platform
                .parse_session_info("my-feature-20250615-123456")
                .unwrap();
            let handler = crate::platform::macos::VSCodeHandler; // Default to VSCode behavior for non-cursor IDEs
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_generate_applescript_contains_expected_elements() {
            use crate::platform::macos::generate_applescript_template;
            let script = generate_applescript_template("Cursor", "my-feature");

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
            use crate::platform::macos::generate_applescript_template;
            let script_cursor = generate_applescript_template("Cursor", "session");
            let script_code = generate_applescript_template("Code", "session");

            assert!(script_cursor.contains("set appName to \"Cursor\""));
            assert!(script_code.contains("set appName to \"Code\""));
        }

        #[test]
        fn test_generate_applescript_different_search_fragments() {
            use crate::platform::macos::generate_applescript_template;
            let script1 = generate_applescript_template("Cursor", "feature-branch");
            let script2 = generate_applescript_template("Cursor", "eager_phoenix");

            assert!(script1.contains("set windowTitleFragment to \"feature-branch\""));
            assert!(script2.contains("set windowTitleFragment to \"eager_phoenix\""));
        }

        #[test]
        fn test_parse_session_info_timestamp_format() {
            let platform = MacOSPlatform;
            let result = platform
                .parse_session_info("my-feature-20250615-123456")
                .unwrap();

            assert_eq!(result.original_id, "my-feature-20250615-123456");

            // Verify the handler generates correct AppleScript
            let handler = crate::platform::macos::CursorHandler;
            let script = handler.generate_applescript(&result);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_parse_session_info_docker_format() {
            let platform = MacOSPlatform;
            let result = platform.parse_session_info("eager_phoenix").unwrap();

            assert_eq!(result.original_id, "eager_phoenix");

            // Verify the handler generates correct AppleScript
            let handler = crate::platform::macos::CursorHandler;
            let script = handler.generate_applescript(&result);
            assert!(script.contains("set windowTitleFragment to \"eager_phoenix\""));
        }

        #[test]
        fn test_parse_session_info_complex_names() {
            let platform = MacOSPlatform;

            // Test with complex Docker-style name
            let result1 = platform.parse_session_info("fix-issue-123_branch").unwrap();
            assert_eq!(result1.original_id, "fix-issue-123_branch");

            // Verify handler works with complex names
            let handler = crate::platform::macos::CursorHandler;
            let script1 = handler.generate_applescript(&result1);
            assert!(script1.contains("set windowTitleFragment to \"fix-issue-123_branch\""));

            // Test with timestamp that has dashes in feature name
            let result2 = platform
                .parse_session_info("fix-bug-123-20250615-123456")
                .unwrap();
            assert_eq!(result2.original_id, "fix-bug-123-20250615-123456");

            // Verify handler works with timestamp format
            let script2 = handler.generate_applescript(&result2);
            assert!(script2.contains("set windowTitleFragment to \"fix-bug-123-20250615-123456\""));
        }

        #[test]
        fn test_get_ide_handler_cursor() {
            let platform = MacOSPlatform;
            let handler = platform.get_ide_handler("cursor");
            assert!(handler.is_ok());
        }

        #[test]
        fn test_get_ide_handler_vscode() {
            let platform = MacOSPlatform;
            let handler1 = platform.get_ide_handler("code");
            let handler2 = platform.get_ide_handler("vscode");
            assert!(handler1.is_ok());
            assert!(handler2.is_ok());
        }

        #[test]
        fn test_get_ide_handler_unsupported() {
            let platform = MacOSPlatform;
            let handler = platform.get_ide_handler("unsupported");
            assert!(handler.is_err());
        }

        #[test]
        fn test_cursor_handler_applescript_generation() {
            use crate::platform::macos::{CursorHandler, IdeHandler, SessionInfo};

            let handler = CursorHandler;

            // Test collision-safe behavior - uses original_id
            let session_info = SessionInfo {
                original_id: "my-feature-20250615-123456".to_string(),
            };

            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set appName to \"Cursor\""));
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_vscode_handler_applescript_generation() {
            use crate::platform::macos::{IdeHandler, SessionInfo, VSCodeHandler};

            let handler = VSCodeHandler;

            // Test that VS Code uses original_id for window matching
            let session_info = SessionInfo {
                original_id: "my-feature-20250615-123456".to_string(),
            };

            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set appName to \"Code\""));
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }

        #[test]
        fn test_format_search_fragment_from_session_info_cursor() {
            use crate::platform::macos::{CursorHandler, IdeHandler, SessionInfo};

            // Test Cursor collision-safe behavior with timestamp session
            let session_info = SessionInfo {
                original_id: "my-feature-20250615-123456".to_string(),
            };
            let handler = CursorHandler;
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));

            // Test Cursor collision-safe behavior with Docker session
            let session_info2 = SessionInfo {
                original_id: "eager_phoenix".to_string(),
            };
            let script2 = handler.generate_applescript(&session_info2);
            assert!(script2.contains("set windowTitleFragment to \"eager_phoenix\""));
        }

        #[test]
        fn test_format_search_fragment_from_session_info_vscode() {
            use crate::platform::macos::{IdeHandler, SessionInfo, VSCodeHandler};

            // Test VS Code uses original_id for collision safety
            let session_info = SessionInfo {
                original_id: "my-feature-20250615-123456".to_string(),
            };
            let handler = VSCodeHandler;
            let script = handler.generate_applescript(&session_info);
            assert!(script.contains("set windowTitleFragment to \"my-feature-20250615-123456\""));
        }
    }
}
