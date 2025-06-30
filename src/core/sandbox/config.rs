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
    pub fn resolve(
        &self,
        cli_sandbox: bool,
        cli_no_sandbox: bool,
        cli_profile: Option<String>,
    ) -> SandboxSettings {
        let default_profile = "permissive-open".to_string();

        // 1. Check CLI flags (highest precedence)
        if cli_no_sandbox {
            return SandboxSettings {
                enabled: false,
                profile: default_profile,
            };
        }

        if cli_sandbox {
            let profile = cli_profile
                .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                .unwrap_or(default_profile.clone());
            return SandboxSettings {
                enabled: true,
                profile,
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

                SandboxSettings {
                    enabled: config.enabled,
                    profile,
                }
            }
            None => SandboxSettings {
                enabled: false,
                profile: cli_profile
                    .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                    .unwrap_or(default_profile),
            },
        }
    }

    /// Validate profile name and return it if valid, otherwise return default
    fn validate_profile(&self, profile: String, source: &str, default: &str) -> String {
        match SandboxProfile::from_name(&profile) {
            Some(_) => profile,
            None => {
                eprintln!(
                    "⚠️  Invalid sandbox profile '{}' from {}, using default '{}'",
                    profile, source, default
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_sandbox_overrides_all() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "restrictive-closed".to_string(),
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
            profile: "permissive-open".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(true, false, Some("restrictive-closed".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive-closed");
    }

    #[test]
    fn test_config_file_settings() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive-closed".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive-closed");
    }

    #[test]
    fn test_cli_profile_overrides_config_profile() {
        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive-open".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, Some("restrictive-closed".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "restrictive-closed");
    }

    #[test]
    fn test_invalid_profile_falls_back_to_default() {
        let config = crate::config::defaults::default_config();
        let resolver = SandboxResolver::new(&config);

        let settings = resolver.resolve(true, false, Some("invalid-profile".to_string()));

        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive-open");
    }

    #[test]
    fn test_no_config_defaults_to_disabled() {
        let config = crate::config::defaults::default_config();
        let resolver = SandboxResolver::new(&config);

        let settings = resolver.resolve(false, false, None);

        assert!(!settings.enabled);
        assert_eq!(settings.profile, "permissive-open");
    }
}
