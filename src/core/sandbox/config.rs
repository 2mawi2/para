use super::profiles::SandboxProfile;
use crate::config::Config;

/// Determines sandbox configuration based on precedence:
/// 1. Command-line flags (highest)
/// 2. Environment variables
/// 3. Config file (lowest)
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
            let profile =
                self.validate_and_get_profile(cli_profile, "CLI argument", &default_profile);
            return SandboxSettings {
                enabled: true,
                profile,
            };
        }

        // 2. Check environment variables
        if let Some(env_sandbox) = Self::get_sandbox_from_env() {
            let profile = if let Some(cli_prof) = cli_profile {
                self.validate_profile(cli_prof, "CLI argument", &default_profile)
            } else if let Some(env_prof) = self.get_profile_from_env() {
                self.validate_profile(env_prof, "PARA_SANDBOX_PROFILE", &default_profile)
            } else if let Some(config) = &self.config {
                self.validate_profile(config.profile.clone(), "config file", &default_profile)
            } else {
                default_profile.clone()
            };

            return SandboxSettings {
                enabled: env_sandbox,
                profile,
            };
        }

        // 3. Use config file settings (lowest precedence)
        match &self.config {
            Some(config) => {
                let profile = if let Some(cli_prof) = cli_profile {
                    self.validate_profile(cli_prof, "CLI argument", &default_profile)
                } else if let Some(env_prof) = self.get_profile_from_env() {
                    self.validate_profile(env_prof, "PARA_SANDBOX_PROFILE", &default_profile)
                } else {
                    self.validate_profile(config.profile.clone(), "config file", &default_profile)
                };

                SandboxSettings {
                    enabled: config.enabled,
                    profile,
                }
            }
            None => SandboxSettings {
                enabled: false,
                profile: cli_profile
                    .map(|p| self.validate_profile(p, "CLI argument", &default_profile))
                    .or_else(|| {
                        self.get_profile_from_env().map(|p| {
                            self.validate_profile(p, "PARA_SANDBOX_PROFILE", &default_profile)
                        })
                    })
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

    /// Helper to validate and get profile with fallback logic
    fn validate_and_get_profile(
        &self,
        cli_profile: Option<String>,
        source: &str,
        default: &str,
    ) -> String {
        cli_profile
            .map(|p| self.validate_profile(p, source, default))
            .or_else(|| {
                self.get_profile_from_env()
                    .map(|p| self.validate_profile(p, "PARA_SANDBOX_PROFILE", default))
            })
            .unwrap_or_else(|| default.to_string())
    }

    fn get_sandbox_from_env() -> Option<bool> {
        match std::env::var("PARA_SANDBOX") {
            Ok(val) => match val.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            Err(_) => None,
        }
    }

    fn get_profile_from_env(&self) -> Option<String> {
        std::env::var("PARA_SANDBOX_PROFILE").ok()
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
    fn test_env_overrides_config() {
        // Clean up any existing env vars first
        std::env::remove_var("PARA_SANDBOX");
        std::env::remove_var("PARA_SANDBOX_PROFILE");
        
        std::env::set_var("PARA_SANDBOX", "true");
        std::env::set_var("PARA_SANDBOX_PROFILE", "permissive-closed");

        let mut config = crate::config::defaults::default_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: false,
            profile: "permissive-open".to_string(),
        });

        let resolver = SandboxResolver::new(&config);
        let settings = resolver.resolve(false, false, None);

        assert!(settings.enabled);
        assert_eq!(settings.profile, "permissive-closed");

        // Clean up
        std::env::remove_var("PARA_SANDBOX");
        std::env::remove_var("PARA_SANDBOX_PROFILE");
    }
}
