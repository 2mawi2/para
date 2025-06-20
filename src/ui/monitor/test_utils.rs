#[cfg(test)]
use crate::config::Config;

#[cfg(test)]
pub fn create_test_config() -> Config {
    // Create a minimal test config without using test_utils
    Config {
        ide: crate::config::IdeConfig {
            name: "test".to_string(),
            command: "echo".to_string(),
            user_data_dir: None,
            wrapper: crate::config::WrapperConfig {
                enabled: false,
                name: String::new(),
                command: String::new(),
            },
        },
        directories: crate::config::DirectoryConfig {
            subtrees_dir: "/tmp/subtrees".to_string(),
            state_dir: "/tmp/.para_state".to_string(),
        },
        git: crate::config::GitConfig {
            branch_prefix: "para".to_string(),
            auto_stage: true,
            auto_commit: false,
        },
        session: crate::config::SessionConfig {
            default_name_format: "%Y%m%d-%H%M%S".to_string(),
            preserve_on_finish: false,
            auto_cleanup_days: Some(7),
        },
    }
}