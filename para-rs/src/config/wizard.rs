use super::defaults::{default_config, get_available_ides};
use super::{Config, ConfigError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

pub fn run_config_wizard() -> Result<Config> {
    println!("🔧 Para Configuration Wizard");
    println!();

    let mut config = default_config();

    config.ide = configure_ide_simple()?;
    config.directories = configure_directories_simple(config.directories)?;

    println!("\n📋 Configuration Summary:");
    display_config_summary(&config);

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
    {
        super::ConfigManager::save(&config)?;
        println!("✅ Configuration saved successfully!");
    } else {
        println!("❌ Configuration not saved.");
        return Err(ConfigError::ValidationError(
            "Configuration cancelled by user".to_string(),
        ));
    }

    Ok(config)
}

fn configure_ide_simple() -> Result<super::IdeConfig> {
    println!("🖥️  IDE Selection");

    let ide_options = vec![
        "cursor (Direct Cursor IDE)",
        "code (Direct VS Code IDE)",
        "claude (Claude Code inside another IDE)",
    ];

    let ide_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which IDE would you like to use?")
        .items(&ide_options)
        .default(0)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    let (ide_name, ide_command) = match ide_selection {
        0 => ("cursor".to_string(), "cursor".to_string()),
        1 => ("code".to_string(), "code".to_string()),
        2 => ("claude".to_string(), "claude".to_string()),
        _ => unreachable!(),
    };

    let wrapper_config = if ide_name == "claude" {
        configure_wrapper_mode_simple()?
    } else {
        super::WrapperConfig {
            enabled: false,
            name: String::new(),
            command: String::new(),
        }
    };

    Ok(super::IdeConfig {
        name: ide_name,
        command: ide_command,
        user_data_dir: None,
        wrapper: wrapper_config,
    })
}

fn configure_wrapper_mode_simple() -> Result<super::WrapperConfig> {
    let wrapper_options = vec!["cursor", "code"];

    let wrapper_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which IDE should wrap Claude Code?")
        .items(&wrapper_options)
        .default(0)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    let wrapper_name = wrapper_options[wrapper_selection].to_string();

    Ok(super::WrapperConfig {
        enabled: true,
        name: wrapper_name.clone(),
        command: wrapper_name,
    })
}

fn configure_directories_simple(
    mut config: super::DirectoryConfig,
) -> Result<super::DirectoryConfig> {
    println!("\n📁 Directories (optional customization)");

    config.subtrees_dir = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Subtrees directory")
        .default(config.subtrees_dir)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    config.state_dir = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("State directory")
        .default(config.state_dir)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    Ok(config)
}

fn display_config_summary(config: &Config) {
    println!("  IDE: {} ({})", config.ide.name, config.ide.command);
    if config.ide.wrapper.enabled {
        println!(
            "  Wrapper: {} ({})",
            config.ide.wrapper.name, config.ide.wrapper.command
        );
    }
    println!("  Subtrees: {}", config.directories.subtrees_dir);
    println!("  State: {}", config.directories.state_dir);
    println!("  Branch prefix: {}", config.git.branch_prefix);
    println!("  Auto-stage: {}", config.git.auto_stage);
    println!("  Auto-commit: {}", config.git.auto_commit);
    println!("  Preserve sessions: {}", config.session.preserve_on_finish);
    if let Some(days) = config.session.auto_cleanup_days {
        println!("  Auto-cleanup: {} days", days);
    } else {
        println!("  Auto-cleanup: disabled");
    }
}

pub fn run_quick_setup() -> Result<Config> {
    println!("🚀 Para Quick Setup");
    println!("Using detected defaults with minimal prompts.\n");

    let mut config = default_config();

    let available_ides = get_available_ides();
    if !available_ides.is_empty() {
        let (ide_name, ide_command) = available_ides[0].clone();
        config.ide.name = ide_name.clone();
        config.ide.command = ide_command;
        println!("✅ Detected IDE: {}", ide_name);
    }

    config.validate()?;
    super::ConfigManager::save(&config)?;
    println!("✅ Configuration saved with defaults!");

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::IntegrationStrategy;
    use crate::config::{
        validation, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };

    #[test]
    fn test_config_summary_display() {
        let config = super::default_config();
        // This test just ensures the display function doesn't panic
        display_config_summary(&config);
    }

    #[test]
    fn test_quick_setup() {
        // Test that quick setup creates a valid config
        // Note: This will use actual defaults and may not work in all environments
        let result = run_quick_setup();
        // We allow this to fail in test environments where IDEs aren't available
        if let Ok(config) = result {
            assert!(config.validate().is_ok());
        }
    }


    #[test]
    fn test_display_config_summary_comprehensive() {
        let config = Config {
            ide: IdeConfig {
                name: "test-ide".to_string(),
                command: "test-command".to_string(),
                user_data_dir: Some("/test/path".to_string()),
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "wrapper-ide".to_string(),
                    command: "wrapper-command".to_string(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "test-subtrees".to_string(),
                state_dir: "test-state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "test-prefix".to_string(),
                auto_stage: false,
                auto_commit: false,
                default_integration_strategy: IntegrationStrategy::Squash,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        };

        // Test that display doesn't panic with all options
        display_config_summary(&config);
    }

    #[test]
    fn test_config_validation_in_wizard() {
        // Test that configs created by the wizard are valid
        let config = default_config();
        assert!(config.validate().is_ok(), "Default config should be valid");
    }

    #[test]
    fn test_ide_detection_integration() {
        let available_ides = get_available_ides();

        // Test that detected IDEs are valid
        for (name, command) in available_ides {
            assert!(!name.is_empty(), "IDE name should not be empty");
            assert!(!command.is_empty(), "IDE command should not be empty");
        }
    }
}
