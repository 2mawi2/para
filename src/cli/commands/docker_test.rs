#[cfg(test)]
mod tests {
    use crate::cli::parser::{SandboxArgs, UnifiedStartArgs};

    #[test]
    fn test_unified_start_docker_image_new_session() {
        // Test that UnifiedStartArgs accepts docker_image for new sessions
        let args = UnifiedStartArgs {
            name_or_session: Some("test".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: Some("custom:latest".to_string()),
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        assert_eq!(args.docker_image, Some("custom:latest".to_string()));
        assert!(!args.no_forward_keys);
    }

    #[test]
    fn test_unified_start_docker_image_with_agent() {
        // Test that UnifiedStartArgs accepts docker_image for agent sessions (old dispatch equivalent)
        let args = UnifiedStartArgs {
            name_or_session: Some("test-session".to_string()),
            prompt: Some("test prompt".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: Some("python:3.11".to_string()),
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        assert_eq!(args.docker_image, Some("python:3.11".to_string()));
        assert!(!args.no_forward_keys);
    }

    #[test]
    fn test_no_forward_keys_flag() {
        // Test the no_forward_keys flag for new session
        let args = UnifiedStartArgs {
            name_or_session: Some("secure".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: Some("untrusted:latest".to_string()),
            no_forward_keys: true,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        assert!(args.no_forward_keys);

        // Test no_forward_keys flag for agent session (old dispatch equivalent)
        let agent_args = UnifiedStartArgs {
            name_or_session: Some("secure-task".to_string()),
            prompt: Some("secure task".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: Some("public:latest".to_string()),
            no_forward_keys: true,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        assert!(agent_args.no_forward_keys);
    }
}
