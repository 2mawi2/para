use crate::cli::parser::IntegrationStrategy;
use serde::{Deserialize, Serialize};

pub mod defaults;
pub mod manager;
pub mod validation;
pub mod wizard;

pub use manager::ConfigManager;
pub use wizard::{run_config_wizard, run_quick_setup};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub ide: IdeConfig,
    pub directories: DirectoryConfig,
    pub git: GitConfig,
    pub session: SessionConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IdeConfig {
    pub name: String,
    pub command: String,
    pub user_data_dir: Option<String>,
    pub wrapper: WrapperConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WrapperConfig {
    pub enabled: bool,
    pub name: String,
    pub command: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DirectoryConfig {
    pub subtrees_dir: String,
    pub state_dir: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitConfig {
    pub branch_prefix: String,
    pub auto_stage: bool,
    pub auto_commit: bool,
    pub default_integration_strategy: IntegrationStrategy,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionConfig {
    pub default_name_format: String,
    pub preserve_on_finish: bool,
    pub auto_cleanup_days: Option<u32>,
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::JsonError(e) => write!(f, "JSON error: {}", e),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        ConfigError::IoError(error)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        ConfigError::JsonError(error)
    }
}

impl Config {
    pub fn load_or_create() -> Result<Self> {
        ConfigManager::load_or_create()
    }

    pub fn validate(&self) -> Result<()> {
        validation::validate_config(self)
    }

    pub fn get_branch_prefix(&self) -> &str {
        &self.git.branch_prefix
    }

    pub fn is_wrapper_enabled(&self) -> bool {
        self.ide.wrapper.enabled
    }

    pub fn get_subtrees_dir(&self) -> &str {
        &self.directories.subtrees_dir
    }

    pub fn get_state_dir(&self) -> &str {
        &self.directories.state_dir
    }

    pub fn should_auto_stage(&self) -> bool {
        self.git.auto_stage
    }

    pub fn should_preserve_on_finish(&self) -> bool {
        self.session.preserve_on_finish
    }

    pub fn get_default_integration_strategy(&self) -> IntegrationStrategy {
        self.git.default_integration_strategy.clone()
    }
}
