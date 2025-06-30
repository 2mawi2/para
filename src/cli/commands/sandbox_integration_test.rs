#[cfg(test)]
mod tests {
    use crate::cli::parser::{DispatchArgs, StartArgs};
    use crate::core::sandbox::config::SandboxResolver;
    use crate::core::sandbox::SandboxConfig;
    use crate::test_utils::test_helpers::create_test_config;

    #[test]
    fn test_sandbox_cli_flags_start_command() {
        // Test that StartArgs accepts all sandbox flags
        let args = StartArgs {
            name: Some("test".to_string()),
            dangerously_skip_permissions: false,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox: true,
            no_sandbox: false,
            sandbox_profile: Some("restrictive-closed".to_string()),
        };

        assert!(args.sandbox);
        assert!(!args.no_sandbox);
        assert_eq!(args.sandbox_profile, Some("restrictive-closed".to_string()));
    }

    #[test]
    fn test_sandbox_cli_flags_dispatch_command() {
        // Test that DispatchArgs accepts all sandbox flags
        let args = DispatchArgs {
            name_or_prompt: Some("test prompt".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox: true,
            no_sandbox: false,
            sandbox_profile: Some("permissive-closed".to_string()),
        };

        assert!(args.sandbox);
        assert!(!args.no_sandbox);
        assert_eq!(args.sandbox_profile, Some("permissive-closed".to_string()));
    }

    #[test]
    fn test_sandbox_resolver_cli_precedence() {
        // CLI flags should override everything
        let mut config = create_test_config();
        config.sandbox = Some(SandboxConfig {
            enabled: false,
            profile: "permissive-open".to_string(),
        });

        // Set env vars (these should be overridden)
        std::env::set_var("PARA_SANDBOX", "false");
        std::env::set_var("PARA_SANDBOX_PROFILE", "permissive-closed");

        let resolver = SandboxResolver::new(&config);

        // Test --sandbox flag overrides
        let settings = resolver.resolve(true, false, Some("restrictive-closed".to_string()));
        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive-closed");

        // Test --no-sandbox flag overrides
        let settings = resolver.resolve(false, true, None);
        assert!(!settings.enabled);

        // Clean up
        std::env::remove_var("PARA_SANDBOX");
        std::env::remove_var("PARA_SANDBOX_PROFILE");
    }

    #[test]
    fn test_sandbox_profiles_validation() {
        use crate::core::sandbox::profiles::SandboxProfile;

        // Valid profiles
        assert!(SandboxProfile::from_name("permissive-open").is_some());
        assert!(SandboxProfile::from_name("permissive-closed").is_some());
        assert!(SandboxProfile::from_name("restrictive-closed").is_some());

        // Invalid profile
        assert!(SandboxProfile::from_name("invalid-profile").is_none());
    }

    #[test]
    fn test_sandbox_environment_variables() {
        // First clean any existing env vars
        std::env::remove_var("PARA_SANDBOX");
        std::env::remove_var("PARA_SANDBOX_PROFILE");
        
        let config = create_test_config();
        let resolver = SandboxResolver::new(&config);

        // Test PARA_SANDBOX=true
        std::env::set_var("PARA_SANDBOX", "true");
        std::env::set_var("PARA_SANDBOX_PROFILE", "permissive-closed");

        let settings = resolver.resolve(false, false, None);
        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive-closed");

        // Test PARA_SANDBOX=false
        std::env::set_var("PARA_SANDBOX", "false");
        let settings = resolver.resolve(false, false, None);
        assert!(!settings.enabled);

        // Test various true/false values
        for val in &["1", "yes", "on", "YES", "On"] {
            std::env::set_var("PARA_SANDBOX", val);
            let settings = resolver.resolve(false, false, None);
            assert!(settings.enabled, "Failed for value: {}", val);
        }

        for val in &["0", "no", "off", "NO", "Off"] {
            std::env::set_var("PARA_SANDBOX", val);
            let settings = resolver.resolve(false, false, None);
            assert!(!settings.enabled, "Failed for value: {}", val);
        }

        // Clean up
        std::env::remove_var("PARA_SANDBOX");
        std::env::remove_var("PARA_SANDBOX_PROFILE");
    }
}
