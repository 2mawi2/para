#[cfg(test)]
mod tests {
    use crate::cli::parser::{DispatchArgs, SandboxArgs, StartArgs};
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
            sandbox_args: SandboxArgs {
                sandbox: true,
                no_sandbox: false,
                sandbox_profile: Some("restrictive".to_string()),
            },
        };

        assert!(args.sandbox_args.sandbox);
        assert!(!args.sandbox_args.no_sandbox);
        assert_eq!(
            args.sandbox_args.sandbox_profile,
            Some("restrictive".to_string())
        );
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
            sandbox_args: SandboxArgs {
                sandbox: true,
                no_sandbox: false,
                sandbox_profile: Some("permissive".to_string()),
            },
        };

        assert!(args.sandbox_args.sandbox);
        assert!(!args.sandbox_args.no_sandbox);
        assert_eq!(
            args.sandbox_args.sandbox_profile,
            Some("permissive".to_string())
        );
    }

    #[test]
    fn test_sandbox_resolver_cli_precedence() {
        // CLI flags should override config
        let mut config = create_test_config();
        config.sandbox = Some(SandboxConfig {
            enabled: false,
            profile: "permissive".to_string(),
        });

        let resolver = SandboxResolver::new(&config);

        // Test --sandbox flag overrides
        let settings = resolver.resolve(true, false, Some("restrictive".to_string()));
        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive");

        // Test --no-sandbox flag overrides
        let settings = resolver.resolve(false, true, None);
        assert!(!settings.enabled);
    }

    #[test]
    fn test_sandbox_profiles_validation() {
        use crate::core::sandbox::profiles::SandboxProfile;

        // Valid profiles
        assert!(SandboxProfile::from_name("permissive").is_some());
        assert!(SandboxProfile::from_name("restrictive").is_some());

        // Legacy names should still work
        assert!(SandboxProfile::from_name("permissive").is_some());
        assert!(SandboxProfile::from_name("restrictive").is_some());

        // Invalid profile
        assert!(SandboxProfile::from_name("invalid-profile").is_none());
    }

    #[test]
    fn test_sandbox_config_based_settings() {
        // Test config-based sandbox settings
        let mut config = create_test_config();

        // Test with sandbox enabled in config
        config.sandbox = Some(SandboxConfig {
            enabled: true,
            profile: "permissive".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);
        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive");

        // Test with sandbox disabled in config
        config.sandbox = Some(SandboxConfig {
            enabled: false,
            profile: "restrictive".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);
        assert!(!settings.enabled);

        // Test with no sandbox config (defaults to disabled)
        config.sandbox = None;
        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);
        assert!(!settings.enabled);
        assert_eq!(settings.profile, "standard");
    }
}
