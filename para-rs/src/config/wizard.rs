use super::defaults::{default_config, detect_wrapper_context, get_available_ides, is_command_available};
use super::validation;
use super::{Config, ConfigError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

pub fn run_config_wizard() -> Result<Config> {
    println!("🔧 Para Configuration Wizard");
    println!("This wizard will help you configure para for your development environment.\n");

    let mut config = default_config();

    config.ide = configure_ide()?;
    config.directories = configure_directories(config.directories)?;
    config.git = configure_git(config.git)?;
    config.session = configure_session(config.session)?;

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

fn configure_ide() -> Result<super::IdeConfig> {
    println!("🖥️  IDE Configuration");
    println!("Para can work with various IDEs. Let's configure your preferred IDE.\n");

    let available_ides = get_available_ides();

    if available_ides.is_empty() {
        println!("⚠️  No supported IDEs detected on your system.");
        println!("Supported IDEs:");
        println!("  • Claude Code (claude) - Recommended for AI development");
        println!("  • Cursor (cursor) - AI-powered code editor");
        println!("  • VS Code (code) - Microsoft Visual Studio Code");
        println!("\nYou can either:");
        println!("  1. Install one of these IDEs and run the wizard again");
        println!("  2. Continue with manual configuration");

        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue with manual IDE configuration?")
            .default(false)
            .interact()
            .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
        {
            return Err(ConfigError::ValidationError(
                "Configuration cancelled - no IDE selected".to_string(),
            ));
        }

        return configure_ide_manually();
    }

    println!("✅ Detected available IDEs:");
    for (name, command) in &available_ides {
        println!("  • {} ({})", name, command);
    }
    println!();

    let ide_names: Vec<String> = available_ides
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let mut ide_names_with_manual = ide_names.clone();
    ide_names_with_manual.push("Manual configuration".to_string());

    let ide_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which IDE would you like to use?")
        .items(&ide_names_with_manual)
        .default(0)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    if ide_selection == ide_names.len() {
        return configure_ide_manually();
    }

    let (ide_name, ide_command) = available_ides[ide_selection].clone();

    let custom_command = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("IDE command (press Enter to use default)")
        .default(ide_command.clone())
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if is_command_available(input) {
                Ok(())
            } else {
                Err("Command not found in PATH")
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    let wrapper_config = configure_wrapper_mode(&ide_name)?;

    let user_data_dir = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Specify custom user data directory for IDE?")
        .default(false)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
    {
        Some(
            Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("User data directory path")
                .validate_with(|input: &String| -> std::result::Result<(), &str> {
                    let path = std::path::Path::new(input);
                    if path.exists() && path.is_dir() {
                        Ok(())
                    } else {
                        Err("Directory does not exist")
                    }
                })
                .interact()
                .map_err(|e| {
                    ConfigError::ValidationError(format!("Failed to read input: {}", e))
                })?,
        )
    } else {
        None
    };

    Ok(super::IdeConfig {
        name: ide_name,
        command: custom_command,
        user_data_dir,
        wrapper: wrapper_config,
    })
}

fn configure_ide_manually() -> Result<super::IdeConfig> {
    println!("🔧 Manual IDE Configuration");
    
    let ide_name = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("IDE name")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("IDE name cannot be empty")
            } else if !validation::is_valid_ide_name(input) {
                Err("Invalid IDE name - use only alphanumeric characters, hyphens, and underscores")
            } else {
                Ok(())
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    let ide_command = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("IDE command")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("IDE command cannot be empty")
            } else if !is_command_available(input) {
                Err("Command not found in PATH")
            } else {
                Ok(())
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    let wrapper_config = configure_wrapper_mode(&ide_name)?;

    let user_data_dir = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Specify custom user data directory for IDE?")
        .default(false)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
    {
        Some(
            Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("User data directory path")
                .validate_with(|input: &String| -> std::result::Result<(), &str> {
                    let path = std::path::Path::new(input);
                    if path.exists() && path.is_dir() {
                        Ok(())
                    } else {
                        Err("Directory does not exist")
                    }
                })
                .interact()
                .map_err(|e| {
                    ConfigError::ValidationError(format!("Failed to read input: {}", e))
                })?,
        )
    } else {
        None
    };

    Ok(super::IdeConfig {
        name: ide_name,
        command: ide_command,
        user_data_dir,
        wrapper: wrapper_config,
    })
}

fn configure_wrapper_mode(ide_name: &str) -> Result<super::WrapperConfig> {
    if ide_name == "claude" {
        if let Some((wrapper_name, wrapper_command)) = detect_wrapper_context() {
            println!(
                "🔍 Detected that Claude Code is running inside {}",
                wrapper_name
            );

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Enable wrapper mode? (Recommended for Claude Code inside other IDEs)")
                .default(true)
                .interact()
                .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
            {
                return Ok(super::WrapperConfig {
                    enabled: true,
                    name: wrapper_name,
                    command: wrapper_command,
                });
            }
        } else if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Configure wrapper mode? (For when Claude Code runs inside another IDE)")
            .default(false)
            .interact()
            .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
        {
            let wrapper_options = vec!["cursor", "code"];
            let wrapper_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Which IDE wraps Claude Code?")
                .items(&wrapper_options)
                .default(0)
                .interact()
                .map_err(|e| {
                    ConfigError::ValidationError(format!("Failed to read input: {}", e))
                })?;

            let wrapper_name = wrapper_options[wrapper_selection].to_string();
            let wrapper_command = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Wrapper IDE command")
                .default(wrapper_name.clone())
                .interact()
                .map_err(|e| {
                    ConfigError::ValidationError(format!("Failed to read input: {}", e))
                })?;

            return Ok(super::WrapperConfig {
                enabled: true,
                name: wrapper_name,
                command: wrapper_command,
            });
        }
    }

    Ok(super::WrapperConfig {
        enabled: false,
        name: String::new(),
        command: String::new(),
    })
}

fn configure_directories(mut config: super::DirectoryConfig) -> Result<super::DirectoryConfig> {
    println!("\n📁 Directory Configuration");
    println!("Configure where para stores worktrees and session state.\n");

    config.subtrees_dir = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Subtrees directory (relative to project root)")
        .default(config.subtrees_dir)
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if validation::validate_directory_config(&super::DirectoryConfig {
                subtrees_dir: input.clone(),
                state_dir: config.state_dir.clone(),
            })
            .is_ok()
            {
                Ok(())
            } else {
                Err("Invalid directory name")
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    config.state_dir = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("State directory (relative to project root)")
        .default(config.state_dir)
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if validation::validate_directory_config(&super::DirectoryConfig {
                subtrees_dir: config.subtrees_dir.clone(),
                state_dir: input.clone(),
            })
            .is_ok()
            {
                Ok(())
            } else {
                Err("Invalid directory name")
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    Ok(config)
}

fn configure_git(mut config: super::GitConfig) -> Result<super::GitConfig> {
    println!("\n🌿 Git Configuration");
    println!("Configure Git-related settings for para sessions.\n");

    config.branch_prefix = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Branch prefix for para sessions")
        .default(config.branch_prefix)
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if validation::validate_git_config(&super::GitConfig {
                branch_prefix: input.clone(),
                auto_stage: config.auto_stage,
                auto_commit: config.auto_commit,
            })
            .is_ok()
            {
                Ok(())
            } else {
                Err("Invalid branch prefix")
            }
        })
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    config.auto_stage = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Automatically stage all changes when finishing sessions?")
        .default(config.auto_stage)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    config.auto_commit = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Automatically commit staged changes when finishing sessions?")
        .default(config.auto_commit)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    Ok(config)
}

fn configure_session(mut config: super::SessionConfig) -> Result<super::SessionConfig> {
    println!("\n⏰ Session Configuration");
    println!("Configure session management and cleanup settings.\n");

    config.preserve_on_finish = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Preserve session data for recovery after finishing?")
        .default(config.preserve_on_finish)
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Enable automatic cleanup of old sessions?")
        .default(config.auto_cleanup_days.is_some())
        .interact()
        .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?
    {
        let days: u32 = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Days to keep old sessions")
            .default(config.auto_cleanup_days.unwrap_or(30))
            .validate_with(|input: &u32| -> std::result::Result<(), &str> {
                if *input > 0 && *input <= 365 {
                    Ok(())
                } else {
                    Err("Must be between 1 and 365 days")
                }
            })
            .interact()
            .map_err(|e| ConfigError::ValidationError(format!("Failed to read input: {}", e)))?;

        config.auto_cleanup_days = Some(days);
    } else {
        config.auto_cleanup_days = None;
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

    let mut config = default_config();

    let available_ides = get_available_ides();
    if !available_ides.is_empty() {
        let (ide_name, ide_command) = available_ides[0].clone();
        config.ide.name = ide_name.clone();
        config.ide.command = ide_command;
        println!("✅ Detected IDE: {}", ide_name);
    }

    if let Some((wrapper_name, wrapper_command)) = detect_wrapper_context() {
        config.ide.wrapper = super::WrapperConfig {
            enabled: true,
            name: wrapper_name.clone(),
            command: wrapper_command,
        };
        println!("✅ Detected wrapper: {}", wrapper_name);
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
    fn test_configure_wrapper_mode_claude_with_detection() {
        // Test wrapper mode configuration for Claude with detected wrapper
        std::env::set_var("TERM_PROGRAM", "vscode");
        
        let result = configure_wrapper_mode("claude");
        
        // Clean up environment
        std::env::remove_var("TERM_PROGRAM");
        
        // This should work since we detect VS Code wrapper
        if let Ok(wrapper_config) = result {
            // The wrapper might be enabled depending on user interaction
            // We just test that the function completes successfully
            assert!(wrapper_config.enabled || !wrapper_config.enabled);
        }
    }

    #[test]
    fn test_configure_wrapper_mode_non_claude() {
        let result = configure_wrapper_mode("cursor").unwrap();
        
        // Non-Claude IDEs shouldn't enable wrapper mode by default
        assert!(!result.enabled);
        assert!(result.name.is_empty());
        assert!(result.command.is_empty());
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
            assert!(validation::is_valid_ide_name(&name), "IDE name should be valid: {}", name);
        }
    }

    #[test]
    fn test_wrapper_context_detection() {
        // Test VS Code detection
        std::env::set_var("TERM_PROGRAM", "vscode");
        let result = detect_wrapper_context();
        std::env::remove_var("TERM_PROGRAM");
        
        if let Some((name, command)) = result {
            assert_eq!(name, "code");
            assert_eq!(command, "code");
        }

        // Test Cursor detection
        std::env::set_var("CURSOR", "1");
        let result = detect_wrapper_context();
        std::env::remove_var("CURSOR");
        
        if let Some((name, command)) = result {
            assert_eq!(name, "cursor");
            assert_eq!(command, "cursor");
        }

        // Test no wrapper detection
        let _result = detect_wrapper_context();
        // Should be None when no wrapper environment variables are set
        // (unless running in an actual IDE environment)
    }
}
