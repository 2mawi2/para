use super::{
    Config, DirectoryConfig, DockerConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
};

pub fn default_config() -> Config {
    Config {
        ide: default_ide_config(),
        directories: default_directory_config(),
        git: default_git_config(),
        session: default_session_config(),
        docker: default_docker_config(),
    }
}

pub fn default_ide_config() -> IdeConfig {
    let detected_ide = detect_ide();

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

pub fn default_docker_config() -> DockerConfig {
    DockerConfig {
        enabled: false,
        mount_workspace: true,
        network_isolation: false, // Default to OFF for backward compatibility
        allowed_domains: default_allowed_domains(),
    }
}

pub fn default_network_isolation() -> bool {
    false // Default to OFF for phased rollout
}

pub fn default_allowed_domains() -> Vec<String> {
    vec![]
}

pub fn detect_ide() -> (String, String) {
    ("claude".to_string(), "claude".to_string())
}

pub fn get_available_ides() -> Vec<(String, String)> {
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
        // Fallback for rare case where directories crate fails
        std::path::PathBuf::from(".").join(".config").join("para")
    }
}

pub fn get_config_file_path() -> std::path::PathBuf {
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
        assert!(config.ide.wrapper.enabled);
        assert!(!config.ide.wrapper.name.is_empty());
        assert!(!config.ide.wrapper.command.is_empty());
    }

    #[test]
    fn test_config_paths() {
        let config_dir = get_default_config_dir();
        assert!(!config_dir.as_os_str().is_empty());

        let config_file = get_config_file_path();
        assert_eq!(
            config_file.file_name().and_then(|n| n.to_str()),
            Some("config.json")
        );
        assert!(config_file.parent().is_some());
    }

    #[test]
    fn test_ide_detection() {
        let available = get_available_ides();
        println!("Available IDEs: {:?}", available);
    }
}
