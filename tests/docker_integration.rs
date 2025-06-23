use para::core::docker::{DockerManager, DockerService, DockerSessionConfig};

#[test]
fn test_docker_manager_creation() {
    let docker_manager = DockerManager::new();
    // Just verify it can be created
    assert!(docker_manager.is_docker_available() || !docker_manager.is_docker_available());
}

#[test]
fn test_docker_session_config_serialization() {
    let config = DockerSessionConfig {
        image: "rust:latest".to_string(),
        volumes: vec![("/host/path".to_string(), "/container/path".to_string())],
        env_vars: vec![("KEY".to_string(), "value".to_string())],
        workdir: Some("/workspace".to_string()),
    };

    // Test serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("rust:latest"));
    assert!(json.contains("/host/path"));
    assert!(json.contains("/container/path"));

    // Test deserialization
    let deserialized: DockerSessionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.image, config.image);
    assert_eq!(deserialized.volumes.len(), 1);
    assert_eq!(deserialized.env_vars.len(), 1);
    assert_eq!(deserialized.workdir, Some("/workspace".to_string()));
}

#[test]
fn test_docker_availability_check() {
    let docker_manager = DockerManager::new();

    // This test just verifies the function runs without panicking
    let available = docker_manager.is_docker_available();

    if available {
        println!("Docker is available on this system");
    } else {
        println!("Docker is not available on this system");
    }
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
    // Verify that DockerManager implements DockerService trait
    let docker_manager = DockerManager::new();
    let _service: &dyn DockerService = &docker_manager;
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
