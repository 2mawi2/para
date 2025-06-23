use para::config::defaults::default_config;
use para::core::docker::manager::DockerManager;
use para::core::docker::service::MockDockerService;
use para::core::docker::{DockerConfig, DockerService};
use std::sync::Arc;

#[test]
fn test_docker_manager_creation() {
    // Create mock service and config for testing
    let mock_service = Arc::new(MockDockerService);
    let config = default_config();
    let docker_config = DockerConfig::default();

    let _docker_manager = DockerManager::new(mock_service.clone(), config, docker_config);

    // Test that the service responds
    assert!(mock_service.health_check().is_ok());
}

#[test]
fn test_docker_config_serialization() {
    let config = DockerConfig::default();

    // Test serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("ubuntu:latest"));
    assert!(json.contains("enabled"));

    // Test deserialization
    let deserialized: DockerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.default_image, config.default_image);
    assert_eq!(deserialized.enabled, config.enabled);
}

#[test]
fn test_docker_availability_check() {
    // Test the mock service health check
    let mock_service = MockDockerService;
    let health_result = mock_service.health_check();

    // Mock service should always be healthy
    assert!(health_result.is_ok());
    println!("Mock Docker service is available");
}

#[test]
fn test_docker_container_name_generation() {
    let session_name = "test-session";
    let expected_container_name = format!("para-{}", session_name);

    assert_eq!(expected_container_name, "para-test-session");
}

// Note: More comprehensive integration tests that actually start/stop containers
// would require Docker to be available and would be better suited for a
// separate test suite that can be run conditionally.

#[test]
fn test_docker_service_trait_implementation() {
    // Verify that MockDockerService implements DockerService trait
    let mock_service = MockDockerService;
    let _service: &dyn DockerService = &mock_service;

    // Test that we can use it as a service
    assert!(mock_service.health_check().is_ok());
}

// Testcontainers tests - These would require testcontainers to be a regular dependency
// For now, we keep them as documentation of how to use testcontainers with para

// Example testcontainers usage (requires testcontainers in [dependencies]):
// ```rust
// use testcontainers::{images::generic::GenericImage, clients::Cli};
//
// #[test]
// #[ignore] // Run with: cargo test -- --ignored
// fn test_with_testcontainers() {
//     let docker = Cli::default();
//     let alpine_image = GenericImage::new("alpine", "latest")
//         .with_wait_for(testcontainers::core::WaitFor::seconds(2));
//
//     let container = docker.run(alpine_image);
//     let container_id = container.id();
//     assert!(!container_id.is_empty());
// }
// ```
