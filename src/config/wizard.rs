use super::defaults::{default_config, get_available_ides};
use super::{Config, ConfigError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

pub fn run_config_wizard() -> Result<Config> {
    println!("🔧 Para Configuration Wizard");
    println!();

    let mut config = default_config();

    config.ide = configure_ide_simple()?;
    config.directories = configure_directories_simple(config.directories)?;
    config.session = configure_session_simple(config.session)?;

    println!("\n📋 Configuration Summary:");
    display_config_summary(&config);

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()
        .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?
    {
        super::ConfigManager::save(&config)?;
        println!("✅ Configuration saved successfully!");
    } else {
        println!("❌ Configuration not saved.");
        return Err(ConfigError::Validation(
            "Configuration cancelled by user".to_string(),
        ));
    }

    Ok(config)
}

fn configure_ide_simple() -> Result<super::IdeConfig> {
    println!("🖥️  IDE Configuration");
    println!("Para works with Claude Code in cloud-based wrapper mode.");

    let ide_name = "claude".to_string();
    let ide_command = "claude".to_string();

    let wrapper_config = configure_wrapper_mode_simple()?;

    Ok(super::IdeConfig {
        name: ide_name,
        command: ide_command,
        user_data_dir: None,
        wrapper: wrapper_config,
    })
}

fn configure_wrapper_mode_simple() -> Result<super::WrapperConfig> {
    let wrapper_options = vec!["code (VS Code)", "cursor (Cursor IDE)"];

    let wrapper_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which IDE wrapper would you like to use for Claude Code?")
        .items(&wrapper_options)
        .default(0)
        .interact()
        .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?;

    let (wrapper_name, wrapper_command) = match wrapper_selection {
        0 => ("code".to_string(), "code".to_string()),
        1 => ("cursor".to_string(), "cursor".to_string()),
        _ => unreachable!(),
    };

    Ok(super::WrapperConfig {
        enabled: true,
        name: wrapper_name,
        command: wrapper_command,
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
        .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?;

    config.state_dir = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("State directory")
        .default(config.state_dir)
        .interact()
        .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?;

    Ok(config)
}

fn configure_session_simple(mut config: super::SessionConfig) -> Result<super::SessionConfig> {
    println!("\n🗂️  Session Management (optional customization)");

    config.preserve_on_finish = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Preserve worktrees after finishing sessions?")
        .default(config.preserve_on_finish)
        .interact()
        .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?;

    if let Some(days) = config.auto_cleanup_days {
        let cleanup_days = Input::<u32>::with_theme(&ColorfulTheme::default())
            .with_prompt("Auto-cleanup preserved sessions after (days)")
            .default(days)
            .validate_with(|input: &u32| {
                if *input > 0 && *input <= 365 {
                    Ok(())
                } else {
                    Err("Please enter a value between 1 and 365 days")
                }
            })
            .interact()
            .map_err(|e| ConfigError::Validation(format!("Failed to read input: {}", e)))?;
        config.auto_cleanup_days = Some(cleanup_days);
    }

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

    let config = default_config();

    let available_ides = get_available_ides();
    if available_ides.is_empty() {
        return Err(ConfigError::Validation(
            "Claude Code is not available. Please install Claude Code and ensure it's in your PATH.".to_string()
        ));
    }

    println!("✅ Detected Claude Code");

    if config.ide.wrapper.enabled {
        println!("✅ Using {} as wrapper", config.ide.wrapper.name);
    }

    config.validate()?;
    super::ConfigManager::save(&config)?;
    println!("✅ Configuration saved with defaults!");

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};

    #[test]
    fn test_config_summary_display() {
        let config = super::default_config();
        display_config_summary(&config);
    }

    #[test]
    fn test_quick_setup() {
        let result = run_quick_setup();
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
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        };

        display_config_summary(&config);
    }

    #[test]
    fn test_config_validation_in_wizard() {
        use crate::config::{
            Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
        };

        let config = Config {
            ide: IdeConfig {
                name: "claude".to_string(),
                command: "claude".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "cursor".to_string(),
                    command: "cursor".to_string(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees/para".to_string(),
                state_dir: ".para_state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: true,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(30),
            },
        };

        assert!(
            config.validate_no_cmd_check().is_ok(),
            "Test config should be valid"
        );
    }

    #[test]
    fn test_ide_detection_integration() {
        let available_ides = get_available_ides();

        for (name, command) in available_ides {
            assert!(!name.is_empty(), "IDE name should not be empty");
            assert!(!command.is_empty(), "IDE command should not be empty");
        }
    }
}
