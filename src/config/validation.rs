use super::{Config, ConfigError, Result};
use std::path::Path;

pub fn validate_config(config: &Config) -> Result<()> {
    validate_ide_config(&config.ide)?;
    validate_directory_config(&config.directories)?;
    validate_git_config(&config.git)?;
    validate_session_config(&config.session)?;
    Ok(())
}

pub fn validate_ide_config(ide: &super::IdeConfig) -> Result<()> {
    validate_ide_config_with_checks(ide, true)
}

#[cfg(test)]
pub fn validate_ide_config_no_cmd_check(ide: &super::IdeConfig) -> Result<()> {
    validate_ide_config_with_checks(ide, false)
}

fn validate_ide_config_with_checks(
    ide: &super::IdeConfig,
    check_command_availability: bool,
) -> Result<()> {
    if ide.name.is_empty() {
        return Err(ConfigError::Validation(
            "IDE name cannot be empty".to_string(),
        ));
    }

    if ide.command.is_empty() {
        return Err(ConfigError::Validation(
            "IDE command cannot be empty".to_string(),
        ));
    }

    if ide.command.to_lowercase() != "claude"
        && ide.command.to_lowercase() != "claude-code"
        && ide.command != "echo"
        && !ide.command.starts_with("echo ")
    {
        return Err(ConfigError::Validation(format!(
            "Para only supports Claude Code. Current IDE: '{}'. Please run 'para config' to configure Claude Code.",
            ide.name
        )));
    }

    if !is_valid_ide_name(&ide.name) {
        return Err(ConfigError::Validation(format!(
            "Invalid IDE name '{}'. Must contain only alphanumeric characters and hyphens",
            ide.name
        )));
    }

    if check_command_availability && !super::defaults::is_command_available(&ide.command) {
        return Err(ConfigError::Validation(format!(
            "Claude Code command '{}' is not available. Please ensure Claude Code is installed and in your PATH",
            ide.command
        )));
    }

    if !ide.wrapper.enabled && ide.command.to_lowercase() == "claude" {
        return Err(ConfigError::Validation(
            "Claude Code requires wrapper mode. Please run 'para config' to enable wrapper mode."
                .to_string(),
        ));
    }

    if ide.wrapper.enabled {
        if ide.wrapper.name.is_empty() {
            return Err(ConfigError::Validation(
                "Wrapper name cannot be empty when wrapper is enabled".to_string(),
            ));
        }
        if ide.wrapper.command.is_empty() {
            return Err(ConfigError::Validation(
                "Wrapper command cannot be empty when wrapper is enabled".to_string(),
            ));
        }

        if ide.wrapper.command != "cursor"
            && ide.wrapper.command != "code"
            && ide.wrapper.command != "echo"
            && !ide.wrapper.command.starts_with("echo ")
        {
            return Err(ConfigError::Validation(format!(
                "Invalid wrapper '{}'. Claude Code requires either 'cursor' or 'code' as wrapper.",
                ide.wrapper.command
            )));
        }

        if check_command_availability
            && !super::defaults::is_command_available(&ide.wrapper.command)
        {
            return Err(ConfigError::Validation(format!(
                "Wrapper command '{}' is not available. Please ensure {} is installed.",
                ide.wrapper.command, ide.wrapper.name
            )));
        }
    }

    Ok(())
}

pub fn validate_directory_config(dirs: &super::DirectoryConfig) -> Result<()> {
    if dirs.subtrees_dir.is_empty() {
        return Err(ConfigError::Validation(
            "Subtrees directory cannot be empty".to_string(),
        ));
    }

    if dirs.state_dir.is_empty() {
        return Err(ConfigError::Validation(
            "State directory cannot be empty".to_string(),
        ));
    }

    if !is_valid_directory_name(&dirs.subtrees_dir) {
        return Err(ConfigError::Validation(format!(
            "Invalid subtrees directory name '{}'. Must be a relative path without '..' components",
            dirs.subtrees_dir
        )));
    }

    if !is_valid_directory_name(&dirs.state_dir) {
        return Err(ConfigError::Validation(format!(
            "Invalid state directory name '{}'. Must be a relative path without '..' components",
            dirs.state_dir
        )));
    }

    Ok(())
}

pub fn validate_git_config(git: &super::GitConfig) -> Result<()> {
    if git.branch_prefix.is_empty() {
        return Err(ConfigError::Validation(
            "Branch prefix cannot be empty".to_string(),
        ));
    }

    if !is_valid_git_ref_name(&git.branch_prefix) {
        return Err(ConfigError::Validation(format!(
            "Invalid branch prefix '{}'. Must be a valid Git reference name",
            git.branch_prefix
        )));
    }

    Ok(())
}

pub fn validate_session_config(session: &super::SessionConfig) -> Result<()> {
    if session.default_name_format.is_empty() {
        return Err(ConfigError::Validation(
            "Default name format cannot be empty".to_string(),
        ));
    }

    if let Some(days) = session.auto_cleanup_days {
        if days == 0 {
            return Err(ConfigError::Validation(
                "Auto cleanup days must be greater than 0".to_string(),
            ));
        }
        if days > 365 {
            return Err(ConfigError::Validation(
                "Auto cleanup days cannot exceed 365".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn is_valid_ide_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

fn is_valid_directory_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('/') {
        return false;
    }

    let path = Path::new(name);
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if let Some(str_component) = os_str.to_str() {
                if str_component == ".." || str_component.contains('\0') {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn is_valid_git_ref_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', ' '];
    if name
        .chars()
        .any(|c| invalid_chars.contains(&c) || c.is_control())
    {
        return false;
    }

    if name.starts_with('/') || name.ends_with('/') || name.contains("..") {
        return false;
    }

    if name.ends_with(".lock") || name.ends_with('.') {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};

    #[test]
    fn test_valid_ide_names() {
        assert!(is_valid_ide_name("cursor"));
        assert!(is_valid_ide_name("code"));
        assert!(is_valid_ide_name("claude"));
        assert!(is_valid_ide_name("my-ide"));
        assert!(is_valid_ide_name("ide_name"));
    }

    #[test]
    fn test_invalid_ide_names() {
        assert!(!is_valid_ide_name(""));
        assert!(!is_valid_ide_name("my ide"));
        assert!(!is_valid_ide_name("ide@name"));
        assert!(!is_valid_ide_name("ide/name"));
    }

    #[test]
    fn test_valid_directory_names() {
        assert!(is_valid_directory_name("subtrees"));
        assert!(is_valid_directory_name("subtrees/para"));
        assert!(is_valid_directory_name(".para_state"));
        assert!(is_valid_directory_name("my-dir/subdir"));
    }

    #[test]
    fn test_invalid_directory_names() {
        assert!(!is_valid_directory_name(""));
        assert!(!is_valid_directory_name("/absolute/path"));
        assert!(!is_valid_directory_name("../parent"));
        assert!(!is_valid_directory_name("dir/../other"));
    }

    #[test]
    fn test_valid_git_ref_names() {
        assert!(is_valid_git_ref_name("para"));
        assert!(is_valid_git_ref_name("feature"));
        assert!(is_valid_git_ref_name("my-branch"));
        assert!(is_valid_git_ref_name("v1.0"));
    }

    #[test]
    fn test_invalid_git_ref_names() {
        assert!(!is_valid_git_ref_name(""));
        assert!(!is_valid_git_ref_name("my branch"));
        assert!(!is_valid_git_ref_name("branch~1"));
        assert!(!is_valid_git_ref_name("branch..other"));
        assert!(!is_valid_git_ref_name("/branch"));
        assert!(!is_valid_git_ref_name("branch/"));
        assert!(!is_valid_git_ref_name("branch.lock"));
    }

    #[test]
    fn test_ide_config_validation() {
        // Valid config - Claude with wrapper
        let valid_config = IdeConfig {
            name: "claude".to_string(),
            command: "claude".to_string(),
            user_data_dir: None,
            wrapper: WrapperConfig {
                enabled: true,
                name: "cursor".to_string(),
                command: "cursor".to_string(),
            },
        };
        assert!(validate_ide_config_no_cmd_check(&valid_config).is_ok());

        // Invalid - non-Claude IDE
        let invalid_config = IdeConfig {
            name: "cursor".to_string(),
            command: "cursor".to_string(),
            user_data_dir: None,
            wrapper: WrapperConfig {
                enabled: true,
                name: "cursor".to_string(),
                command: "cursor".to_string(),
            },
        };
        assert!(validate_ide_config_no_cmd_check(&invalid_config).is_err());

        // Invalid - Claude without wrapper
        let invalid_no_wrapper = IdeConfig {
            name: "claude".to_string(),
            command: "claude".to_string(),
            user_data_dir: None,
            wrapper: WrapperConfig {
                enabled: false,
                name: String::new(),
                command: String::new(),
            },
        };
        assert!(validate_ide_config_no_cmd_check(&invalid_no_wrapper).is_err());
    }

    #[test]
    fn test_directory_config_validation() {
        let valid_config = DirectoryConfig {
            subtrees_dir: "subtrees/para".to_string(),
            state_dir: ".para_state".to_string(),
        };
        assert!(validate_directory_config(&valid_config).is_ok());

        let invalid_config = DirectoryConfig {
            subtrees_dir: "/absolute/path".to_string(),
            state_dir: ".para_state".to_string(),
        };
        assert!(validate_directory_config(&invalid_config).is_err());
    }

    #[test]
    fn test_git_config_validation() {
        let valid_config = GitConfig {
            branch_prefix: "para".to_string(),
            auto_stage: true,
            auto_commit: true,
        };
        assert!(validate_git_config(&valid_config).is_ok());

        let invalid_config = GitConfig {
            branch_prefix: "my branch".to_string(),
            auto_stage: true,
            auto_commit: true,
        };
        assert!(validate_git_config(&invalid_config).is_err());
    }

    #[test]
    fn test_session_config_validation() {
        let valid_config = SessionConfig {
            default_name_format: "%Y%m%d-%H%M%S".to_string(),
            preserve_on_finish: true,
            auto_cleanup_days: Some(30),
        };
        assert!(validate_session_config(&valid_config).is_ok());

        let invalid_config = SessionConfig {
            default_name_format: "".to_string(),
            preserve_on_finish: true,
            auto_cleanup_days: Some(0),
        };
        assert!(validate_session_config(&invalid_config).is_err());
    }
}
