use super::profiles::SandboxProfile;
use crate::config::Config;

/// Determines sandbox configuration based on precedence:
/// 1. Command-line flags (highest)
/// 2. Config file (lowest)
pub struct SandboxResolver {
    config: Option<crate::core::sandbox::SandboxConfig>,
}

impl SandboxResolver {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.sandbox.clone(),
        }
    }

    /// Resolve sandbox settings with precedence
    #[allow(dead_code)] // Used in tests and as convenience method
    pub fn resolve(
        &self,
        cli_sandbox: bool,
        cli_no_sandbox: bool,
        cli_profile: Option<String>,
    ) -> SandboxSettings {
        self.resolve_with_domains(cli_sandbox, cli_no_sandbox, cli_profile, Vec::new())
    }

    /// Resolve sandbox settings with precedence and allowed domains
    pub fn resolve_with_domains(
        &self,
        cli_sandbox: bool,
        cli_no_sandbox: bool,
        cli_profile: Option<String>,
        cli_allowed_domains: Vec<String>,
    ) -> SandboxSettings {
        let default_profile = "standard".to_string();

        // 1. Check CLI flags (highest precedence)
        if cli_no_sandbox {
            return SandboxSettings {
                enabled: false,
                profile: default_profile,
                allowed_domains: Vec::new(),
                network_sandbox: false,
            };
        }

        if cli_sandbox {
            let profile = cli_profile
                .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                .unwrap_or(default_profile.clone());

            // Merge CLI and config allowed_domains
            let mut allowed_domains = self
                .config
                .as_ref()
                .map(|c| c.allowed_domains.clone())
                .unwrap_or_default();
            allowed_domains.extend(cli_allowed_domains);
            allowed_domains.sort();
            allowed_domains.dedup();

            return SandboxSettings {
                enabled: true,
                profile: profile.clone(),
                allowed_domains,
                network_sandbox: profile == "standard-proxied",
            };
        }

        // 2. Use config file settings
        match &self.config {
            Some(config) => {
                let profile = cli_profile
                    .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                    .unwrap_or_else(|| {
                        self.validate_profile(
                            config.profile.clone(),
                            "config file",
                            &default_profile,
                        )
                    });

                // Merge CLI and config allowed_domains
                let mut allowed_domains = config.allowed_domains.clone();
                allowed_domains.extend(cli_allowed_domains);
                allowed_domains.sort();
                allowed_domains.dedup();

                SandboxSettings {
                    enabled: config.enabled,
                    profile: profile.clone(),
                    allowed_domains,
                    network_sandbox: config.enabled && profile == "standard-proxied",
                }
            }
            None => {
                let profile = cli_profile
                    .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                    .unwrap_or(default_profile);
                SandboxSettings {
                    enabled: false,
                    profile: profile.clone(),
                    allowed_domains: cli_allowed_domains,
                    network_sandbox: false, // network_sandbox requires sandbox to be enabled
                }
            }
        }
    }

    /// Resolve sandbox settings with network sandboxing support
    pub fn resolve_with_network(
        &self,
        cli_sandbox: bool,
        cli_no_sandbox: bool,
        cli_profile: Option<String>,
        cli_network_sandbox: bool,
        cli_allowed_domains: Vec<String>,
    ) -> SandboxSettings {
        // Network sandboxing implies sandboxing is enabled with a specific profile
        if cli_network_sandbox {
            // Merge CLI and config allowed_domains
            let mut allowed_domains = self
                .config
                .as_ref()
                .map(|c| c.allowed_domains.clone())
                .unwrap_or_default();
            allowed_domains.extend(cli_allowed_domains);
            allowed_domains.sort();
            allowed_domains.dedup();

            return SandboxSettings {
                enabled: true,
                profile: "standard-proxied".to_string(),
                allowed_domains,
                network_sandbox: true,
            };
        }

        // Otherwise use regular resolution
        self.resolve_with_domains(
            cli_sandbox,
            cli_no_sandbox,
            cli_profile,
            cli_allowed_domains,
        )
    }

    /// Validate profile name and return it if valid, otherwise return default
    fn validate_profile(&self, profile: String, source: &str, default: &str) -> String {
        match SandboxProfile::from_name(&profile) {
            Some(_) => profile,
            None => {
                eprintln!(
                    "⚠️  Invalid sandbox profile '{profile}' from {source}, using default '{default}'"
                );
                default.to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxSettings {
    pub enabled: bool,
    pub profile: String,
    pub allowed_domains: Vec<String>,
    pub network_sandbox: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_sandbox_overrides_all() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "restrictive".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, true, None);

        assert!(!settings.enabled);
    }

    #[test]
    fn test_cli_sandbox_overrides_config() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: false,
            profile: "permissive".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(true, false, Some("restrictive".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive");
    }

    #[test]
    fn test_config_file_settings() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive");
    }

    #[test]
    fn test_cli_profile_overrides_config_profile() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive".to_string(),
            allowed_domains: vec![],
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, Some("restrictive".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive");
    }

    #[test]
    fn test_invalid_profile_falls_back_to_default() {
        let config = crate::config::defaults::default_config();
        let resolver = SandboxResolver::new(&config);

        let settings = resolver.resolve(true, false, Some("invalid-profile".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard");
    }

    #[test]
    fn test_no_config_defaults_to_disabled() {
        let config = crate::config::defaults::default_config();
        let resolver = SandboxResolver::new(&config);

        let settings = resolver.resolve(false, false, None);

        assert!(!settings.enabled);
        assert_eq!(settings.profile, "standard");
    }

    #[test]
    fn test_resolve_with_domains_merges_correctly() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "standard".to_string(),
            allowed_domains: vec!["github.com".to_string(), "internal.com".to_string()],
        });

        let resolver = SandboxResolver::new(&config);
        let cli_domains = vec!["npmjs.org".to_string(), "github.com".to_string()];
        let settings = resolver.resolve_with_domains(true, false, None, cli_domains);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard");

        // Should have merged and deduplicated domains
        let domains = &settings.allowed_domains;
        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&"github.com".to_string()));
        assert!(domains.contains(&"internal.com".to_string()));
        assert!(domains.contains(&"npmjs.org".to_string()));
    }

    #[test]
    fn test_resolve_with_network_preserves_domains() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: false,
            profile: "permissive".to_string(),
            allowed_domains: vec!["api.company.com".to_string()],
        });

        let resolver = SandboxResolver::new(&config);
        let cli_domains = vec!["external.api.com".to_string()];
        let settings = resolver.resolve_with_network(false, false, None, true, cli_domains);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "standard-proxied");

        // Should have both config and CLI domains
        let domains = &settings.allowed_domains;
        assert_eq!(domains.len(), 2);
        assert!(domains.contains(&"api.company.com".to_string()));
        assert!(domains.contains(&"external.api.com".to_string()));
    }

    #[test]
    fn test_resolve_with_domains_no_config() {
        let config = crate::config::defaults::default_config();
        let resolver = SandboxResolver::new(&config);
        let cli_domains = vec!["test.com".to_string()];

        let settings = resolver.resolve_with_domains(
            true,
            false,
            Some("restrictive".to_string()),
            cli_domains,
        );

        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive");
        assert_eq!(settings.allowed_domains, vec!["test.com".to_string()]);
    }
}
