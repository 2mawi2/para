pub mod cleanup;
pub mod config;
pub mod launcher;
pub mod profiles;
pub mod proxy;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_profile")]
    pub profile: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: default_profile(),
        }
    }
}

fn default_profile() -> String {
    "permissive-open".to_string()
}

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod network_sandbox_test;
