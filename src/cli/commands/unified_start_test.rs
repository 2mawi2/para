#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::{SandboxArgs, UnifiedStartArgs};
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    fn create_test_args(
        name_or_session: Option<&str>,
        prompt: Option<&str>,
        sandbox_no_network: bool,
        allowed_domains: Vec<&str>,
    ) -> UnifiedStartArgs {
        UnifiedStartArgs {
            name_or_session: name_or_session.map(String::from),
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
    fn test_single_arg_with_spaces_should_be_treated_as_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // This should be treated as a prompt, not a session name
        let args = create_test_args(
            Some("please download golem.de"),
            None,
            true,
            vec!["golem.de"],
        );

        let result = determine_intent(&args, &session_manager);
        
        // Currently this fails because it's treated as an invalid session name
        assert!(result.is_ok(), "Should treat text with spaces as prompt");
        
        match result.unwrap() {
            StartIntent::NewWithAgent { name, prompt } => {
                assert!(name.is_none(), "Should auto-generate session name");
                assert_eq!(prompt, "please download golem.de");
            }
            _ => panic!("Expected NewWithAgent intent"),
        }
    }

    #[test]
    fn test_valid_session_name_is_treated_as_session_name() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Valid session name should be treated as such
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
    fn test_prompt_with_network_sandbox_args() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Explicit session name + prompt should work
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
    fn test_single_word_prompts_are_ambiguous() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Single word could be either session name or prompt
        let args = create_test_args(
            Some("download"),
            None,
            false,
            vec![],
        );

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_ok());
        
        // Currently treated as session name
        match result.unwrap() {
            StartIntent::NewInteractive { name } => {
                assert_eq!(name, Some("download".to_string()));
            }
            _ => panic!("Expected NewInteractive intent"),
        }
    }

    #[test]
    fn test_prompt_detection_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = setup_isolated_test_environment(&temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Text with special chars should be treated as prompt
        let test_cases = vec![
            "implement the TODO items",
            "fix bug #123",
            "add feature: user auth",
            "what is 2+2?",
            "analyze website.com",
        ];

        for test_prompt in test_cases {
            let args = create_test_args(
                Some(test_prompt),
                None,
                false,
                vec![],
            );

            let result = determine_intent(&args, &session_manager);
            
            // These should be treated as prompts, not session names
            assert!(result.is_ok(), "Failed for prompt: {}", test_prompt);
            
            match result {
                Ok(StartIntent::NewWithAgent { prompt, .. }) => {
                    assert_eq!(prompt, test_prompt);
                }
                _ => panic!("Expected NewWithAgent for: {}", test_prompt),
            }
        }
    }

    #[test]
    fn test_explicit_prompt_flag_would_be_nice() {
        // This test demonstrates what we'd like to have
        // Currently there's no --prompt flag in UnifiedStartArgs
        
        // Ideal API:
        // para start --prompt "download something" --sandbox-no-network
        // This would remove ambiguity
        
        // For now, we have to use workarounds:
        // 1. Two args: para start session-name "prompt text"
        // 2. File: para start --file prompt.txt
        // 3. Stdin: echo "prompt" | para start
    }
}