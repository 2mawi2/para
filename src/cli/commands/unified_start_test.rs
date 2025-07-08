#[cfg(test)]
mod tests {
    use crate::cli::commands::unified_start::{determine_intent, StartIntent};
    use crate::cli::parser::{SandboxArgs, UnifiedStartArgs};
    use crate::config::Config;
    use crate::core::session::SessionManager;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    fn create_test_args(
        name: Option<&str>,
        prompt: Option<&str>,
        sandbox_no_network: bool,
        allowed_domains: Vec<&str>,
    ) -> UnifiedStartArgs {
        UnifiedStartArgs {
            name: name.map(String::from),
            prompt: prompt.map(String::from),
            file: None,
            dangerously_skip_permissions: true,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network,
                allowed_domains: allowed_domains.into_iter().map(String::from).collect(),
            },
        }
    }

    #[test]
    fn test_invalid_session_name_with_spaces_fails() {
        // Names with spaces are invalid session names
        let args = create_test_args(
            Some("please download golem.de"),
            None,
            true,
            vec!["golem.de"],
        );

        let result = args.validate();
        assert!(result.is_err(), "Session name with spaces should fail validation");
        assert!(result.unwrap_err().to_string().contains("Session name can only contain"));
    }

    #[test]
    fn test_valid_session_name_creates_interactive_session() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Valid session name without prompt creates interactive session
        let args = create_test_args(
            Some("download-task"),
            None,
            true,
            vec!["golem.de"],
        );

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_ok());
        
        match result.unwrap() {
            StartIntent::NewInteractive { name } => {
                assert_eq!(name, Some("download-task".to_string()));
            }
            _ => panic!("Expected NewInteractive intent"),
        }
    }

    #[test]
    fn test_prompt_flag_with_name_creates_agent_session() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Using -p flag with session name creates agent session
        let args = create_test_args(
            Some("my-task"),
            Some("download golem.de and analyze it"),
            true,
            vec!["golem.de", "example.com"],
        );

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_ok());
        
        match result.unwrap() {
            StartIntent::NewWithAgent { name, prompt } => {
                assert_eq!(name, Some("my-task".to_string()));
                assert_eq!(prompt, "download golem.de and analyze it");
            }
            _ => panic!("Expected NewWithAgent intent"),
        }
    }

    #[test]
    fn test_no_args_creates_interactive_session() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // No arguments creates interactive session
        let args = create_test_args(
            None,
            None,
            false,
            vec![],
        );

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_ok());
        
        match result.unwrap() {
            StartIntent::NewInteractive { name } => {
                assert_eq!(name, None);
            }
            _ => panic!("Expected NewInteractive intent"),
        }
    }

    #[test]
    fn test_prompt_flag_required_for_ai_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Using -p flag creates AI session
        let test_prompts = vec![
            "implement the TODO items",
            "fix bug #123",
            "add feature: user auth",
            "what is 2+2?",
            "analyze website.com",
        ];

        for test_prompt in test_prompts {
            let args = create_test_args(
                None,
                Some(test_prompt),
                false,
                vec![],
            );

            let result = determine_intent(&args, &session_manager);
            assert!(result.is_ok(), "Failed for prompt: {}", test_prompt);
            
            match result.unwrap() {
                StartIntent::NewWithAgent { name, prompt } => {
                    assert_eq!(name, None);
                    assert_eq!(prompt, test_prompt);
                }
                _ => panic!("Expected NewWithAgent for: {}", test_prompt),
            }
        }
    }

    #[test]
    fn test_file_flag_creates_ai_session() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Create a test file
        let prompt_file = temp_dir.path().join("test_prompt.txt");
        std::fs::write(&prompt_file, "Test prompt from file").unwrap();

        // Using -f flag creates AI session
        let mut args = create_test_args(
            None,
            None,
            false,
            vec![],
        );
        args.file = Some(prompt_file);

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_ok());
        
        match result.unwrap() {
            StartIntent::NewWithAgent { name, prompt } => {
                assert_eq!(name, None);
                assert_eq!(prompt, "Test prompt from file");
            }
            _ => panic!("Expected NewWithAgent intent"),
        }
    }
}