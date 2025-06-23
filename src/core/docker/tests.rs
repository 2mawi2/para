//! Tests for Docker integration

use super::*;
use crate::core::docker::config::{detect_project_type, ProjectType};
use crate::core::docker::manager::DockerManager;
use crate::core::docker::service::ContainerStats;
use crate::core::docker::session::MountType;
use crate::core::session::state::SessionStatus;
use crate::core::session::SessionState;
use crate::test_utils::test_helpers::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Mock implementation of DockerService for testing
struct MockDockerService {
    containers: Arc<Mutex<HashMap<String, ContainerSession>>>,
    images: Arc<Mutex<Vec<String>>>,
    health_check_result: bool,
}

impl MockDockerService {
    fn new() -> Self {
        Self {
            containers: Arc::new(Mutex::new(HashMap::new())),
            images: Arc::new(Mutex::new(vec!["ubuntu:latest".to_string()])),
            health_check_result: true,
        }
    }

    fn with_failing_health_check() -> Self {
        Self {
            containers: Arc::new(Mutex::new(HashMap::new())),
            images: Arc::new(Mutex::new(vec![])),
            health_check_result: false,
        }
    }
}

impl DockerService for MockDockerService {
    fn create_session(
        &self,
        session_name: &str,
        config: &DockerConfig,
        working_dir: &Path,
    ) -> DockerResult<ContainerSession> {
        let container_id = format!("mock-container-{}", session_name);
        let mut session = ContainerSession::new(
            container_id.clone(),
            session_name.to_string(),
            config.default_image.clone(),
            working_dir.to_path_buf(),
        );

        session.add_para_labels();

        self.containers
            .lock()
            .unwrap()
            .insert(session_name.to_string(), session.clone());

        Ok(session)
    }

    fn start_session(&self, session_name: &str) -> DockerResult<()> {
        let mut containers = self.containers.lock().unwrap();
        match containers.get_mut(session_name) {
            Some(container) => {
                if container.is_running() {
                    return Err(DockerError::ContainerAlreadyRunning {
                        name: session_name.to_string(),
                    });
                }
                container.status = ContainerStatus::Running;
                container.started_at = Some(chrono::Utc::now());
                Ok(())
            }
            None => Err(DockerError::ContainerNotFound {
                name: session_name.to_string(),
            }),
        }
    }

    fn stop_session(&self, session_name: &str) -> DockerResult<()> {
        let mut containers = self.containers.lock().unwrap();
        match containers.get_mut(session_name) {
            Some(container) => {
                if !container.is_running() {
                    return Err(DockerError::ContainerNotRunning {
                        name: session_name.to_string(),
                    });
                }
                container.status = ContainerStatus::Stopped;
                container.stopped_at = Some(chrono::Utc::now());
                Ok(())
            }
            None => Err(DockerError::ContainerNotFound {
                name: session_name.to_string(),
            }),
        }
    }

    fn finish_session(&self, session_name: &str, remove: bool) -> DockerResult<()> {
        // Try to stop the session, but ignore ContainerNotRunning errors
        match self.stop_session(session_name) {
            Ok(_) => {}
            Err(DockerError::ContainerNotRunning { .. }) => {
                // Container is already stopped, that's fine
            }
            Err(e) => return Err(e),
        }

        if remove {
            self.containers.lock().unwrap().remove(session_name);
        }
        Ok(())
    }

    fn cancel_session(&self, session_name: &str) -> DockerResult<()> {
        self.finish_session(session_name, true)
    }

    fn get_container_status(&self, session_name: &str) -> DockerResult<ContainerStatus> {
        self.containers
            .lock()
            .unwrap()
            .get(session_name)
            .map(|c| c.status.clone())
            .ok_or_else(|| DockerError::ContainerNotFound {
                name: session_name.to_string(),
            })
    }

    fn health_check(&self) -> DockerResult<()> {
        if self.health_check_result {
            Ok(())
        } else {
            Err(DockerError::DaemonNotAvailable(
                "Mock daemon not available".to_string(),
            ))
        }
    }

    fn ensure_image(&self, image: &str) -> DockerResult<()> {
        let mut images = self.images.lock().unwrap();
        if !images.contains(&image.to_string()) {
            images.push(image.to_string());
        }
        Ok(())
    }

    fn list_sessions(&self) -> DockerResult<Vec<ContainerSession>> {
        Ok(self.containers.lock().unwrap().values().cloned().collect())
    }

    // Stub implementations for other methods
    fn exec_in_container(
        &self,
        _session_name: &str,
        _command: &str,
        _args: &[String],
        _env: Option<HashMap<String, String>>,
    ) -> DockerResult<String> {
        Ok("Mock output".to_string())
    }

    fn attach_to_container(&self, _session_name: &str) -> DockerResult<()> {
        Ok(())
    }

    fn get_logs(
        &self,
        _session_name: &str,
        _follow: bool,
        _tail: Option<usize>,
    ) -> DockerResult<String> {
        Ok("Mock logs".to_string())
    }

    fn copy_to_container(
        &self,
        _session_name: &str,
        _src: &Path,
        _dest: &Path,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn copy_from_container(
        &self,
        _session_name: &str,
        _src: &Path,
        _dest: &Path,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn update_resources(
        &self,
        _session_name: &str,
        _cpu_limit: Option<f64>,
        _memory_limit: Option<u64>,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn get_stats(&self, _session_name: &str) -> DockerResult<ContainerStats> {
        Ok(ContainerStats {
            cpu_usage_percent: 25.0,
            memory_usage_bytes: 1073741824,
            memory_limit_bytes: 4294967296,
            network_rx_bytes: 1024,
            network_tx_bytes: 2048,
            block_read_bytes: 0,
            block_write_bytes: 0,
        })
    }

    fn wait_for_status(
        &self,
        session_name: &str,
        expected_status: ContainerStatus,
        _timeout_seconds: u64,
    ) -> DockerResult<()> {
        let status = self.get_container_status(session_name)?;
        if status == expected_status {
            Ok(())
        } else {
            Err(DockerError::CommunicationError(
                "Status mismatch".to_string(),
            ))
        }
    }
}

#[test]
fn test_docker_config_default() {
    let config = DockerConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.default_image, "ubuntu:latest");
    assert!(config.image_mappings.contains_key(&ProjectType::Rust));
}

#[test]
fn test_docker_config_serialization() {
    let config = DockerConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: DockerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.default_image, deserialized.default_image);
}

#[test]
fn test_container_session_creation() {
    let session = ContainerSession::new(
        "container-123".to_string(),
        "test-session".to_string(),
        "rust:latest".to_string(),
        PathBuf::from("/workspace"),
    );

    assert_eq!(session.container_id, "container-123");
    assert_eq!(session.session_name, "test-session");
    assert_eq!(session.status, ContainerStatus::Created);
    assert!(!session.is_running());
    assert!(session.can_start());
}

#[test]
fn test_project_type_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Test Rust detection
    std::fs::write(temp_dir.path().join("Cargo.toml"), "").unwrap();
    assert_eq!(detect_project_type(temp_dir.path()), ProjectType::Rust);

    // Test Node detection
    std::fs::remove_file(temp_dir.path().join("Cargo.toml")).unwrap();
    std::fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
    assert_eq!(detect_project_type(temp_dir.path()), ProjectType::Node);
}

#[test]
fn test_docker_manager_container_lifecycle() {
    let git_temp = TempDir::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

    let mut config = create_test_config();
    config.directories.state_dir = temp_dir
        .path()
        .join(".para_state")
        .to_string_lossy()
        .to_string();

    // Ensure state directory exists
    std::fs::create_dir_all(&config.directories.state_dir).unwrap();

    let docker_config = DockerConfig::default();
    let mock_service = Arc::new(MockDockerService::new());
    let manager = DockerManager::new(mock_service.clone(), config, docker_config);

    // Create a test session state
    let session_state = SessionState {
        name: "test-session".to_string(),
        branch: "para/test-session".to_string(),
        worktree_path: temp_dir.path().to_path_buf(),
        created_at: chrono::Utc::now(),
        status: SessionStatus::Active,
        task_description: None,
        last_activity: None,
        git_stats: None,
        session_type: crate::core::session::SessionType::Container { container_id: None },
        is_docker: None,
    };

    // Test container creation
    let container = manager
        .create_container_for_session(&session_state)
        .unwrap();
    assert_eq!(container.session_name, "test-session");
    assert_eq!(container.status, ContainerStatus::Created);

    // Test container start
    manager.start_container("test-session").unwrap();
    let status = manager.sync_container_status("test-session").unwrap();
    assert_eq!(status, ContainerStatus::Running);

    // Test container stop
    manager.stop_container("test-session").unwrap();
    let status = manager.sync_container_status("test-session").unwrap();
    assert_eq!(status, ContainerStatus::Stopped);

    // Test container finish with removal
    manager.finish_container("test-session", true).unwrap();
    let result = mock_service.get_container_status("test-session");
    assert!(matches!(result, Err(DockerError::ContainerNotFound { .. })));
}

#[test]
fn test_docker_manager_error_handling() {
    let git_temp = TempDir::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

    let mut config = create_test_config();
    config.directories.state_dir = temp_dir
        .path()
        .join(".para_state")
        .to_string_lossy()
        .to_string();

    // Ensure state directory exists
    std::fs::create_dir_all(&config.directories.state_dir).unwrap();

    let docker_config = DockerConfig::default();
    let mock_service = Arc::new(MockDockerService::new());
    let manager = DockerManager::new(mock_service, config, docker_config);

    // Test starting non-existent container
    let result = manager.start_container("non-existent");
    assert!(matches!(result, Err(DockerError::ContainerNotFound { .. })));

    // Test stopping non-existent container
    let result = manager.stop_container("non-existent");
    assert!(matches!(result, Err(DockerError::ContainerNotFound { .. })));
}

#[test]
fn test_docker_health_check() {
    let mock_service = MockDockerService::new();
    assert!(mock_service.health_check().is_ok());

    let failing_service = MockDockerService::with_failing_health_check();
    assert!(matches!(
        failing_service.health_check(),
        Err(DockerError::DaemonNotAvailable(_))
    ));
}

#[test]
fn test_resource_limits() {
    let limits = ResourceLimits {
        cpu_limit: Some(2.5),
        memory_limit: Some(4294967296), // 4GB
        memory_swap_limit: None,
        cpu_shares: Some(1024),
        blkio_weight: Some(500),
        pids_limit: Some(1000),
    };

    assert_eq!(limits.cpu_limit, Some(2.5));
    assert_eq!(limits.memory_limit, Some(4294967296));
}

#[test]
fn test_volume_mapping_variable_expansion() {
    let mapping = VolumeMapping {
        source: "$HOME/.config".to_string(),
        target: "/root/.config".to_string(),
        read_only: true,
        mount_type: MountType::Bind,
    };

    // In actual implementation, variable expansion would happen
    assert!(mapping.source.contains("$HOME"));
    assert!(mapping.read_only);
}
