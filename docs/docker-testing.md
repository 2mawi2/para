# Docker Testing Strategy for Para

This document outlines the testing strategy for Docker integration in the Para project.

## Overview

Para's Docker functionality allows sessions to run inside isolated Docker containers, providing a clean development environment for each parallel workflow. The testing infrastructure supports both unit tests using mocks and integration tests using real Docker containers.

## Test Structure

### 1. Unit Tests with Mocks

Located in: `src/core/docker/mod.rs` (unit tests) and test files using mocks

**Purpose**: Test business logic without requiring Docker to be installed

**Key Components**:
- `MockDockerService`: A mock implementation of the `DockerService` trait
- Simulates container lifecycle operations
- Tracks container state in memory
- No actual Docker daemon required

**Example**:
```rust
use para::test_utils::docker_helpers::{mock_docker_service, create_test_docker_config};

#[test]
fn test_container_lifecycle() {
    let mock = mock_docker_service();
    let config = create_test_docker_config();
    
    // Start container
    mock.start_container("test-session", &config).unwrap();
    assert!(mock.is_container_running("test-session").unwrap());
    
    // Stop container
    mock.stop_container("test-session").unwrap();
    assert!(!mock.is_container_running("test-session").unwrap());
}
```

### 2. Integration Tests

Located in: `tests/docker_integration.rs`

**Purpose**: Test actual Docker operations when Docker is available

**Key Components**:
- Tests that interact with real Docker daemon
- Conditional tests that skip when Docker is unavailable
- Testcontainers-based tests for complex scenarios

**Running Integration Tests**:
```bash
# Run all tests (skips Docker tests if daemon not available)
just test

# Run Docker integration tests specifically
cargo test --test docker_integration

# Run ignored testcontainers tests (requires Docker)
cargo test --test docker_integration -- --ignored
```

## Test Utilities

### DockerTestHelper

Located in: `src/test_utils/docker_helpers.rs`

Provides utilities for Docker testing:
- `is_docker_available()`: Check if Docker daemon is running
- `ensure_image()`: Pull Docker images for tests
- `create_test_container()`: Create containers with unique names
- `DockerTestGuard`: RAII guard for automatic cleanup

### Helper Functions

- `create_test_docker_config()`: Creates a standard test configuration
- `mock_docker_service()`: Creates a mock Docker service
- `setup_test_container_environment()`: Sets up a test environment with cleanup
- `verify_container_cleanup()`: Verifies containers were properly removed

## Testing Patterns

### 1. Skip Tests When Docker Unavailable

```rust
#[test]
fn test_requiring_docker() {
    if !DockerTestHelper::is_docker_available() {
        eprintln!("Skipping test - Docker not available");
        return;
    }
    
    // Test implementation
}
```

### 2. Use Test Guards for Cleanup

```rust
#[test]
fn test_with_cleanup() {
    let mut guard = DockerTestGuard::new();
    let container_name = "test-container";
    guard.add_container(container_name.to_string());
    
    // Test operations...
    // Container automatically cleaned up when guard drops
}
```

### 3. Mock Complex Scenarios

```rust
#[test]
fn test_container_failure() {
    let mut mock = mock_docker_service();
    mock.with_container("existing", true);
    
    // Test duplicate container creation
    let result = mock.start_container("existing", &config);
    assert!(result.is_err());
}
```

## Testcontainers Integration

For complex integration scenarios, use testcontainers:

```rust
use testcontainers::{clients::Cli, images::generic::GenericImage};

#[test]
#[ignore] // Run explicitly with --ignored flag
fn test_with_real_container() {
    let docker = Cli::default();
    let image = GenericImage::new("alpine", "latest");
    let container = docker.run(image);
    
    // Test with real container
    // Automatic cleanup on drop
}
```

## CI/CD Considerations

### GitHub Actions

Tests run in CI with Docker available:
```yaml
- name: Run tests
  run: just test
  
- name: Run Docker integration tests
  run: cargo test --test docker_integration -- --ignored
```

### Local Development

Developers without Docker can still:
- Run unit tests with mocks
- Test non-Docker functionality
- Review Docker-related code changes

## Best Practices

1. **Always use mocks for unit tests** - Don't require Docker for basic functionality tests
2. **Guard Docker operations** - Check availability before running Docker commands
3. **Clean up resources** - Use `DockerTestGuard` or manual cleanup
4. **Unique naming** - Use timestamps or UUIDs for container names to avoid conflicts
5. **Conditional compilation** - Use `#[cfg(test)]` for test-only code
6. **Clear error messages** - Indicate when tests skip due to missing Docker

## Future Enhancements

1. **Docker Compose Support**: Test multi-container scenarios
2. **Volume Testing**: Test persistent data across container restarts
3. **Network Testing**: Test container networking configurations
4. **Performance Tests**: Measure container startup/shutdown times
5. **Security Tests**: Verify container isolation and permissions

## Troubleshooting

### Common Issues

1. **"Docker daemon not running"**
   - Start Docker Desktop or Docker daemon
   - Tests will skip gracefully if Docker unavailable

2. **"Container name already exists"**
   - Previous test didn't clean up properly
   - Run: `docker ps -a | grep para-test | awk '{print $1}' | xargs docker rm -f`

3. **"Permission denied"**
   - Add user to docker group: `sudo usermod -aG docker $USER`
   - Log out and back in for changes to take effect

4. **Testcontainers timeout**
   - Increase timeout in tests
   - Check Docker daemon responsiveness
   - Ensure sufficient system resources

### Debug Commands

```bash
# List all para test containers
docker ps -a | grep para-test

# View container logs
docker logs <container-name>

# Clean up all test containers
docker ps -a | grep para-test | awk '{print $1}' | xargs docker rm -f

# Check Docker daemon status
docker version
docker info
```