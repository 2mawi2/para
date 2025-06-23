#[cfg(test)]
mod tests {
    use crate::config::defaults::{default_allowed_domains, default_network_isolation};
    use crate::config::DockerConfig;

    #[test]
    fn test_default_network_isolation_enabled() {
        assert!(default_network_isolation());
    }

    #[test]
    fn test_default_allowed_domains_empty() {
        let domains = default_allowed_domains();
        assert!(domains.is_empty());
    }

    #[test]
    fn test_docker_config_network_isolation_defaults() {
        use crate::config::defaults::default_docker_config;

        let config = default_docker_config();
        assert!(config.network_isolation);
        assert!(config.allowed_domains.is_empty());
    }

    #[test]
    fn test_docker_config_with_custom_domains() {
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: true,
            allowed_domains: vec!["custom-api.com".to_string(), "my-service.com".to_string()],
        };

        assert!(config.network_isolation);
        assert_eq!(config.allowed_domains.len(), 2);
        assert!(config
            .allowed_domains
            .contains(&"custom-api.com".to_string()));
        assert!(config
            .allowed_domains
            .contains(&"my-service.com".to_string()));
    }

    #[test]
    fn test_docker_config_disable_network_isolation() {
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: false,
            allowed_domains: vec![],
        };

        assert!(!config.network_isolation);
    }
}
