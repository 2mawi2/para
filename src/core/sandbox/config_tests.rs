#[cfg(test)]
mod tests {
    use super::super::config::SandboxResolver;
    use crate::core::sandbox::SandboxConfig;

    #[test]
    fn test_network_sandbox_from_config_profile() {
        // Create a config with standard-proxied profile
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(SandboxConfig {
            enabled: true,
            profile: "standard-proxied".to_string(),
            allowed_domains: vec!["api.example.com".to_string()],
        });

        let resolver = SandboxResolver::new(&config);

        // Resolve without any CLI flags
        let settings = resolver.resolve_with_network(
            false,  // cli_sandbox
            false,  // cli_no_sandbox
            None,   // cli_profile
            false,  // cli_network_sandbox
            vec![], // cli_allowed_domains
        );

        // Verify that network_sandbox is set based on profile
        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard-proxied");
        assert!(
            settings.network_sandbox,
            "network_sandbox should be true when profile is standard-proxied"
        );
    }

    #[test]
    fn test_network_sandbox_cli_override() {
        // Create a config with standard profile (not proxied)
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(SandboxConfig {
            enabled: true,
            profile: "standard".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);

        // Resolve with CLI network sandbox flag
        let settings = resolver.resolve_with_network(
            false,  // cli_sandbox
            false,  // cli_no_sandbox
            None,   // cli_profile
            true,   // cli_network_sandbox - this should override!
            vec![], // cli_allowed_domains
        );

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard-proxied");
        assert!(
            settings.network_sandbox,
            "CLI network_sandbox flag should set network_sandbox to true"
        );
    }

    #[test]
    fn test_project_config_network_sandbox() {
        // Simulate merged config from project with standard-proxied
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(SandboxConfig {
            enabled: true,
            profile: "standard-proxied".to_string(),
            allowed_domains: vec!["github.com".to_string(), "api.internal.com".to_string()],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve_with_network(false, false, None, false, vec![]);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard-proxied");
        assert_eq!(settings.allowed_domains.len(), 2);
        assert!(
            settings.network_sandbox,
            "Project config with standard-proxied should set network_sandbox to true"
        );
    }

    #[test]
    fn test_standard_profile_no_network_sandbox() {
        // Create a config with standard profile (not proxied)
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(SandboxConfig {
            enabled: true,
            profile: "standard".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve_with_network(false, false, None, false, vec![]);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard");
        assert!(
            !settings.network_sandbox,
            "Standard profile should not enable network_sandbox"
        );
    }

    #[test]
    fn test_disabled_sandbox_no_network_sandbox() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(SandboxConfig {
            enabled: false,
            profile: "standard-proxied".to_string(),
            allowed_domains: vec!["example.com".to_string()],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve_with_network(false, false, None, false, vec![]);

        assert!(!settings.enabled);
        assert_eq!(settings.profile, "standard-proxied");
        assert!(
            !settings.network_sandbox,
            "network_sandbox should be false when sandbox is disabled"
        );
        assert_eq!(settings.allowed_domains, vec!["example.com"]);
    }
}
