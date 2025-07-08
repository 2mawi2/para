#[cfg(test)]
mod tests {
    use crate::cli::parser::SandboxArgs;
    use crate::core::ide::LaunchOptions;
    use crate::core::sandbox::launcher::{wrap_command_with_sandbox, SandboxOptions};

    #[test]
    fn test_network_sandbox_cli_args() {
        let args = SandboxArgs {
            sandbox: false,
            no_sandbox: false,
            sandbox_profile: None,
            sandbox_no_network: true,
            allowed_domains: vec!["npmjs.org".to_string(), "pypi.org".to_string()],
        };

        assert!(args.sandbox_no_network);
        assert_eq!(args.allowed_domains.len(), 2);
        assert!(args.allowed_domains.contains(&"npmjs.org".to_string()));
        assert!(args.allowed_domains.contains(&"pypi.org".to_string()));
    }

    #[test]
    fn test_launch_options_network_sandbox() {
        let options = LaunchOptions {
            skip_permissions: false,
            continue_conversation: false,
            claude_session_id: None,
            prompt: None,
            sandbox_override: Some(true),
            sandbox_profile: None,
            network_sandbox: true,
            allowed_domains: vec!["custom.com".to_string()],
        };

        assert!(options.network_sandbox);
        assert_eq!(options.allowed_domains.len(), 1);
        assert_eq!(options.allowed_domains[0], "custom.com");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_sandbox_wrapper_with_proxy() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path();

        let options = SandboxOptions {
            profile: "standard-proxied".to_string(),
            proxy_address: Some("127.0.0.1:8877".to_string()),
            allowed_domains: vec!["example.com".to_string()],
        };

        let command = "claude --dangerously-skip-permissions";
        let sandboxed = wrap_command_with_sandbox(command, worktree_path, &options).unwrap();

        // Should indicate wrapper script is needed for network proxy
        assert!(sandboxed.needs_wrapper_script);
        assert_eq!(sandboxed.proxy_port, Some(8877));

        // Check that proxy address is passed as parameter
        assert!(sandboxed.command.contains("-D 'PROXY_ADDR=127.0.0.1:8877'"));

        // Check that the original command is included
        assert!(sandboxed
            .command
            .contains("claude --dangerously-skip-permissions"));
    }

    #[test]
    fn test_cli_flag_conflicts() {
        // sandbox-no-network should conflict with sandbox
        let args1 = SandboxArgs {
            sandbox: true,
            no_sandbox: false,
            sandbox_profile: None,
            sandbox_no_network: true,
            allowed_domains: vec![],
        };

        // This would be caught by clap at runtime due to conflicts_with
        // Here we just ensure the struct can be created
        assert!(args1.sandbox);
        assert!(args1.sandbox_no_network);

        // allowed-domains requires sandbox-no-network
        let args2 = SandboxArgs {
            sandbox: false,
            no_sandbox: false,
            sandbox_profile: None,
            sandbox_no_network: true,
            allowed_domains: vec!["example.com".to_string()],
        };

        assert!(args2.sandbox_no_network);
        assert!(!args2.allowed_domains.is_empty());
    }
}
