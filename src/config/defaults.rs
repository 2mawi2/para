use super::{Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};

pub fn default_config() -> Config {
    Config {
        ide: default_ide_config(),
        directories: default_directory_config(),
        git: default_git_config(),
        session: default_session_config(),
    }
}

pub fn default_ide_config() -> IdeConfig {
    let detected_ide = detect_ide();

    // Claude Code requires a wrapper (cursor or code)
    // Default to cursor if available, otherwise code
    let wrapper_command = if is_command_available("cursor") {
        "cursor"
    } else if is_command_available("code") {
        "code"
    } else {
        "cursor" // fallback
    };

    IdeConfig {
        name: detected_ide.0.clone(),
        command: detected_ide.1,
        user_data_dir: None,
        wrapper: WrapperConfig {
            enabled: true,
            name: wrapper_command.to_string(),
            command: wrapper_command.to_string(),
        },
    }
}

pub fn default_directory_config() -> DirectoryConfig {
    DirectoryConfig {
        subtrees_dir: ".para/worktrees".to_string(),
        state_dir: ".para/state".to_string(),
    }
}

pub fn default_git_config() -> GitConfig {
    GitConfig {
        branch_prefix: "para".to_string(),
        auto_stage: true,
        auto_commit: true,
    }
}

pub fn default_session_config() -> SessionConfig {
    SessionConfig {
        default_name_format: "%Y%m%d-%H%M%S".to_string(),
        preserve_on_finish: false,
        auto_cleanup_days: Some(30),
    }
}

pub fn detect_ide() -> (String, String) {
    // Para only supports Claude Code
    ("claude".to_string(), "claude".to_string())
}

pub fn get_available_ides() -> Vec<(String, String)> {
    // Para only supports Claude Code
    if is_command_available("claude") {
        vec![("claude".to_string(), "claude".to_string())]
    } else {
        vec![]
    }
}

pub fn is_command_available(command: &str) -> bool {
    if cfg!(target_os = "windows") {
        std::process::Command::new("where")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

pub fn get_default_config_dir() -> std::path::PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "para") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        std::env::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".config")
            .join("para")
    }
}

pub fn get_config_file_path() -> std::path::PathBuf {
    // Allow environment variable override for config path (used in tests)
    if let Ok(config_path) = std::env::var("PARA_CONFIG_PATH") {
        return std::path::PathBuf::from(config_path);
    }

    get_default_config_dir().join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = default_config();
        assert_eq!(config.git.branch_prefix, "para");
        assert_eq!(config.directories.subtrees_dir, ".para/worktrees");
        assert_eq!(config.directories.state_dir, ".para/state");
        assert!(config.git.auto_stage);
        assert!(!config.session.preserve_on_finish);
        // Verify wrapper is enabled by default
        assert!(config.ide.wrapper.enabled);
        assert!(!config.ide.wrapper.name.is_empty());
        assert!(!config.ide.wrapper.command.is_empty());
    }

    #[test]
    fn test_config_paths() {
        // This test checks basic properties of config paths without modifying global state
        // to ensure it's safe for parallel test execution

        // Test that get_config_file_path returns a valid path
        let config_file = get_config_file_path();
        assert!(config_file.ends_with("config.json"));
        assert!(config_file.parent().is_some());

        // Test that get_default_config_dir returns a valid directory
        let config_dir = get_default_config_dir();
        // The directory path should exist or be creatable
        assert!(!config_dir.as_os_str().is_empty());

        // If no env var is set, the config file should be under the config dir
        // But we can't test this reliably in parallel tests since another test
        // might have set PARA_CONFIG_PATH
    }

    #[test]
    fn test_ide_detection() {
        let available = get_available_ides();
        // Note: This test allows empty IDE lists for CI environments
        // where IDEs might not be installed
        println!("Available IDEs: {:?}", available);
    }
}
