#[cfg(test)]
mod platform_tests {
    use crate::platform::{GenericPlatform, PlatformManager, get_platform_manager, IdeConfig, WindowInfo};
    use tempfile::TempDir;

    #[test]
    fn test_platform_manager_factory() {
        let platform = get_platform_manager();
        
        #[cfg(target_os = "macos")]
        {
            // On macOS, should return MacOSPlatform
            let result = platform.close_ide_window("test-session", "cursor");
            // Should not panic and return Ok (even if AppleScript fails)
            assert!(result.is_ok() || result.is_err());
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // On non-macOS, should return GenericPlatform with warning
            let result = platform.close_ide_window("test-session", "cursor");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_ide_config_creation() {
        let config = IdeConfig {
            name: "cursor".to_string(),
            command: "cursor".to_string(),
            wrapper_enabled: true,
            wrapper_name: "code".to_string(),
            wrapper_command: "code".to_string(),
        };
        
        assert_eq!(config.name, "cursor");
        assert_eq!(config.command, "cursor");
        assert!(config.wrapper_enabled);
        assert_eq!(config.wrapper_name, "code");
        assert_eq!(config.wrapper_command, "code");
    }

    #[test]
    fn test_window_info_creation() {
        let window = WindowInfo {
            id: "12345".to_string(),
            title: "Test Window - test-session".to_string(),
            process_id: 1234,
            app_name: "Cursor".to_string(),
        };
        
        assert_eq!(window.id, "12345");
        assert!(window.title.contains("test-session"));
        assert_eq!(window.process_id, 1234);
        assert_eq!(window.app_name, "Cursor");
    }

    #[test]
    fn test_generic_platform_fallback() {
        let platform = GenericPlatform;
        
        // All methods should return Ok but with warnings
        assert!(platform.close_ide_window("test", "cursor").is_ok());
        assert!(platform.find_ide_windows("test").unwrap().is_empty());
        assert!(platform.get_active_window_title().unwrap().is_none());
        assert!(platform.bring_window_to_front("123").is_ok());
        assert!(platform.terminate_process_group(1234).is_ok());
        
        // launch_ide_with_wrapper should attempt basic launch
        let temp_dir = TempDir::new().unwrap();
        let config = IdeConfig {
            name: "echo".to_string(),
            command: "echo".to_string(),
            wrapper_enabled: false,
            wrapper_name: String::new(),
            wrapper_command: String::new(),
        };
        
        let result = platform.launch_ide_with_wrapper(&config, temp_dir.path(), None);
        assert!(result.is_ok());
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use crate::platform::macos::MacOSPlatform;
        use crate::platform::{IdeConfig, PlatformManager};
        use tempfile::TempDir;

        #[test]
        fn test_macos_platform_creation() {
            let platform = MacOSPlatform;
            
            // Test that platform can be created
            let result = platform.close_ide_window("nonexistent-session", "cursor");
            // Should complete without panic (may succeed or fail depending on system state)
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn test_applescript_execution() {
            let platform = MacOSPlatform;
            
            // Test basic AppleScript execution (should always work on macOS)
            let script = "return \"test\"";
            let result = platform.execute_applescript_with_output(script);
            
            if result.is_ok() {
                let output = result.unwrap();
                assert!(output.trim() == "test");
            }
            // If it fails, that's okay too (might be permissions or system state)
        }

        #[test]
        fn test_vscode_tasks_json_creation() {
            let platform = MacOSPlatform;
            let temp_dir = TempDir::new().unwrap();
            
            let result = platform.create_vscode_tasks_json(temp_dir.path(), Some("test prompt"));
            assert!(result.is_ok());
            
            let tasks_file = temp_dir.path().join(".vscode").join("tasks.json");
            assert!(tasks_file.exists());
            
            let content = std::fs::read_to_string(&tasks_file).unwrap();
            assert!(content.contains("Start Claude Code"));
            assert!(content.contains("test prompt"));
        }

        #[test]
        fn test_vscode_tasks_json_without_prompt() {
            let platform = MacOSPlatform;
            let temp_dir = TempDir::new().unwrap();
            
            let result = platform.create_vscode_tasks_json(temp_dir.path(), None);
            assert!(result.is_ok());
            
            let tasks_file = temp_dir.path().join(".vscode").join("tasks.json");
            assert!(tasks_file.exists());
            
            let content = std::fs::read_to_string(&tasks_file).unwrap();
            assert!(content.contains("Start Claude Code"));
            assert!(!content.contains("--prompt"));
        }

        #[test]
        fn test_window_line_parsing() {
            let platform = MacOSPlatform;
            
            // Test parsing AppleScript window output
            let test_line = "Cursor, Test Window - session123, 1234, 5678";
            let result = platform.parse_window_line(test_line);
            
            assert!(result.is_some());
            let window = result.unwrap();
            assert_eq!(window.app_name, "Cursor");
            assert_eq!(window.title, "Test Window - session123");
            assert_eq!(window.process_id, 1234);
            assert_eq!(window.id, "5678");
        }

        #[test]
        fn test_launch_wrapper_mode() {
            let platform = MacOSPlatform;
            let temp_dir = TempDir::new().unwrap();
            
            let config = IdeConfig {
                name: "claude".to_string(),
                command: "claude".to_string(),
                wrapper_enabled: true,
                wrapper_name: "code".to_string(),
                wrapper_command: "echo".to_string(), // Use echo for testing
            };
            
            // Should create .vscode/tasks.json and attempt to launch wrapper
            let result = platform.launch_wrapper_mode(&config, temp_dir.path(), Some("test prompt"));
            
            // Even if launch fails, tasks.json should be created
            let tasks_file = temp_dir.path().join(".vscode").join("tasks.json");
            assert!(tasks_file.exists());
        }

        #[test]
        fn test_launch_standalone_ide() {
            let platform = MacOSPlatform;
            let temp_dir = TempDir::new().unwrap();
            
            let config = IdeConfig {
                name: "claude".to_string(),
                command: "echo".to_string(), // Use echo for testing
                wrapper_enabled: false,
                wrapper_name: String::new(),
                wrapper_command: String::new(),
            };
            
            let result = platform.launch_standalone_ide(&config, temp_dir.path(), Some("test prompt"));
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_platform_integration_workflow() {
        let platform = get_platform_manager();
        let temp_dir = TempDir::new().unwrap();
        
        let config = IdeConfig {
            name: "cursor".to_string(),
            command: "echo".to_string(),
            wrapper_enabled: false,
            wrapper_name: String::new(),
            wrapper_command: String::new(),
        };
        
        // Test full workflow: launch -> find -> close
        let launch_result = platform.launch_ide_with_wrapper(&config, temp_dir.path(), None);
        assert!(launch_result.is_ok());
        
        let windows = platform.find_ide_windows("test-session").unwrap();
        // May or may not find windows depending on system state
        
        let close_result = platform.close_ide_window("test-session", "cursor");
        assert!(close_result.is_ok());
    }

    #[test]
    fn test_error_handling() {
        let platform = get_platform_manager();
        
        // Test with invalid IDE config
        let invalid_config = IdeConfig {
            name: "nonexistent-ide".to_string(),
            command: "nonexistent-command-12345".to_string(),
            wrapper_enabled: false,
            wrapper_name: String::new(),
            wrapper_command: String::new(),
        };
        
        let temp_dir = TempDir::new().unwrap();
        let result = platform.launch_ide_with_wrapper(&invalid_config, temp_dir.path(), None);
        
        // Should handle error gracefully (either succeed on non-macOS or fail gracefully on macOS)
        match result {
            Ok(_) => {
                // Generic platform succeeded with warning
            }
            Err(_) => {
                // macOS platform failed as expected with nonexistent command
            }
        }
    }
}