use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::config::Config;
use crate::core::session::SessionManager;

/// Manages automatic cleanup of orphaned Docker containers
pub struct ContainerCleaner {
    config: Config,
}

impl ContainerCleaner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Check if cleanup should run based on marker file age
    pub fn should_run_cleanup(&self) -> Result<bool> {
        let cleanup_marker =
            PathBuf::from(&self.config.directories.state_dir).join(".last_container_cleanup");

        match cleanup_marker.metadata() {
            Ok(metadata) => {
                let age = SystemTime::now()
                    .duration_since(metadata.modified()?)
                    .unwrap_or(Duration::MAX);
                Ok(age > Duration::from_secs(3600)) // 1 hour
            }
            Err(_) => Ok(true), // File doesn't exist, should run
        }
    }

    /// Update the cleanup marker file
    pub fn update_cleanup_marker(&self) -> Result<()> {
        let cleanup_marker =
            PathBuf::from(&self.config.directories.state_dir).join(".last_container_cleanup");
        std::fs::write(&cleanup_marker, "")?;
        Ok(())
    }

    /// Trigger cleanup if needed, runs in background
    pub fn maybe_cleanup_async(&self) -> Result<()> {
        if !self.should_run_cleanup()? {
            return Ok(());
        }

        // Clone config for the background thread
        let config = self.config.clone();

        // Spawn background cleanup
        thread::spawn(move || {
            let cleaner = ContainerCleaner::new(config);
            if let Err(e) = cleaner.cleanup_orphaned_containers() {
                eprintln!("Background container cleanup error: {}", e);
            }
        });

        // Update marker immediately to prevent multiple runs
        self.update_cleanup_marker()?;
        Ok(())
    }

    /// Cleanup orphaned containers
    pub fn cleanup_orphaned_containers(&self) -> Result<()> {
        // List all para containers
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "name=para-",
                "--format",
                "{{.Names}}",
            ])
            .output()?;

        if !output.status.success() {
            // Docker not available or command failed, skip silently
            return Ok(());
        }

        let container_names = String::from_utf8_lossy(&output.stdout);
        let session_manager = SessionManager::new(&self.config);

        for container_name in container_names.lines() {
            if let Some(session_name) = container_name.strip_prefix("para-") {
                // Check if session exists
                if !session_manager.session_exists(session_name) {
                    // Session doesn't exist, remove container
                    self.remove_container(container_name);
                }
            }
        }

        Ok(())
    }

    /// Remove a single container (fire and forget)
    fn remove_container(&self, container_name: &str) {
        Command::new("docker")
            .args(["rm", "-f", container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok(); // Don't fail if this doesn't work
    }

    /// Parse session name from container name
    pub fn parse_session_from_container(container_name: &str) -> Option<String> {
        container_name.strip_prefix("para-").map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_session_from_container() {
        assert_eq!(
            ContainerCleaner::parse_session_from_container("para-my-session"),
            Some("my-session".to_string())
        );
        assert_eq!(
            ContainerCleaner::parse_session_from_container("other-container"),
            None
        );
    }

    #[test]
    fn test_should_run_cleanup_no_marker() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = crate::test_utils::test_helpers::create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        // Ensure state directory exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let cleaner = ContainerCleaner::new(config);

        // No marker file exists
        assert!(cleaner.should_run_cleanup().unwrap());
    }

    #[test]
    fn test_should_run_cleanup_with_marker() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = crate::test_utils::test_helpers::create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        // Ensure state directory exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let cleaner = ContainerCleaner::new(config);

        // Create marker file
        cleaner.update_cleanup_marker().unwrap();

        // Should not run immediately after update
        assert!(!cleaner.should_run_cleanup().unwrap());
    }

    #[test]
    fn test_update_cleanup_marker() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = crate::test_utils::test_helpers::create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        // Ensure state directory exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let cleaner = ContainerCleaner::new(config);

        // Update marker
        cleaner.update_cleanup_marker().unwrap();

        // Verify marker exists
        let marker_path =
            PathBuf::from(&cleaner.config.directories.state_dir).join(".last_container_cleanup");
        assert!(marker_path.exists());
    }
}
