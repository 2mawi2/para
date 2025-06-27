#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::config::{
        Config, DirectoryConfig, DockerConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };

    fn create_test_config_with_docker(docker_image: Option<String>) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: ".para/worktrees".to_string(),
                state_dir: ".para/state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
            docker: docker_image.map(|image| DockerConfig {
                setup_script: None,
                default_image: Some(image),
                forward_env_keys: None,
            }),
            setup_script: None,
        }
    }

    #[test]
    fn test_docker_image_priority_cli_flag() {
        // CLI flag should take highest priority
        let config = create_test_config_with_docker(Some("config-image:latest".to_string()));
        let manager =
            DockerManager::with_image(config, false, vec![], Some("cli-image:latest".to_string()));

        // We can't directly test get_docker_image since it's private,
        // but we can verify the manager was created with the right image
        assert_eq!(manager.docker_image, Some("cli-image:latest".to_string()));
    }

    #[test]
    fn test_docker_image_priority_config() {
        // Config should be used when no CLI flag
        let config = create_test_config_with_docker(Some("config-image:latest".to_string()));
        let manager = DockerManager::with_image(config, false, vec![], None);

        assert_eq!(manager.docker_image, None);
    }

    #[test]
    fn test_docker_image_priority_default() {
        // Should fall back to default when neither CLI nor config is set
        let config = create_test_config_with_docker(None);
        let manager = DockerManager::new(config, false, vec![]);

        assert_eq!(manager.docker_image, None);
    }

    #[test]
    fn test_docker_manager_new_compatibility() {
        // Ensure the old new() method still works
        let config = create_test_config_with_docker(None);
        let manager = DockerManager::new(config.clone(), true, vec!["test.com".to_string()]);

        assert!(manager.network_isolation);
        assert_eq!(manager.allowed_domains, vec!["test.com".to_string()]);
        assert_eq!(manager.docker_image, None);
    }

    #[test]
    fn test_docker_manager_forward_keys() {
        // Test that forward_keys defaults to true for backward compatibility
        let config = create_test_config_with_docker(None);
        let manager = DockerManager::new(config, false, vec![]);

        assert!(manager.forward_keys);

        // Test with_image also defaults to true
        let config2 = create_test_config_with_docker(None);
        let manager2 =
            DockerManager::with_image(config2, false, vec![], Some("test:latest".to_string()));

        assert!(manager2.forward_keys);

        // Test with_options can set it to false
        let config3 = create_test_config_with_docker(None);
        let manager3 = DockerManager::with_options(config3, false, vec![], None, false);

        assert!(!manager3.forward_keys);
    }
}
