//! Docker authentication setup module
//!
//! This module handles the interactive authentication flow for Claude containers,
//! creating authenticated Docker images that can be reused across sessions.

use super::{DockerError, DockerResult};
use std::process::Command;

/// Manages Docker authentication setup for Claude containers
pub struct DockerAuthSetup;

impl DockerAuthSetup {
    /// Create a Docker volume for storing authentication data
    pub fn create_auth_volume(&self) -> DockerResult<String> {
        let user_id = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let volume_name = format!("para-auth-claude-{}", user_id);

        // Check if volume already exists
        let exists_output = Command::new("docker")
            .args(["volume", "inspect", &volume_name])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !exists_output.status.success() {
            // Create new volume
            let create_output = Command::new("docker")
                .args(["volume", "create", &volume_name])
                .output()
                .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

            if !create_output.status.success() {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Failed to create auth volume: {}",
                    String::from_utf8_lossy(&create_output.stderr)
                )));
            }
        }

        Ok(volume_name)
    }

    /// Set up interactive authentication by starting a container with mounted volume
    pub fn setup_interactive_auth(&self) -> DockerResult<()> {
        let volume_name = self.create_auth_volume()?;
        let container_name = "para-auth-setup-temp";

        // Remove any existing auth setup container
        let _ = Command::new("docker")
            .args(["rm", "-f", container_name])
            .output();

        // Create container with auth volume
        let create_output = Command::new("docker")
            .args([
                "create",
                "--name",
                container_name,
                "-v",
                &format!("{}:/root/.config", volume_name),
                "-it",
                "para-claude:latest",
                "bash",
            ])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !create_output.status.success() {
            return Err(DockerError::ContainerCreationFailed(
                String::from_utf8_lossy(&create_output.stderr).to_string(),
            ));
        }

        // Start the container
        let start_output = Command::new("docker")
            .args(["start", container_name])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !start_output.status.success() {
            return Err(DockerError::ContainerStartFailed(
                String::from_utf8_lossy(&start_output.stderr).to_string(),
            ));
        }

        println!("\n🔐 Please authenticate with Claude in the container:");
        println!("   Running: docker exec -it {} claude /login", container_name);
        println!("   Follow the prompts to complete authentication.\n");

        // Run interactive authentication
        let exec_status = Command::new("docker")
            .args(["exec", "-it", container_name, "claude", "/login"])
            .status()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to exec claude /login: {}", e)))?;

        if !exec_status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Authentication failed or was cancelled"
            )));
        }

        // Stop and remove the temporary container
        let _ = Command::new("docker")
            .args(["stop", container_name])
            .output();
        let _ = Command::new("docker")
            .args(["rm", container_name])
            .output();

        println!("✅ Authentication completed successfully!");

        Ok(())
    }

    /// Create an authenticated snapshot image after successful authentication
    pub fn create_authenticated_snapshot(&self) -> DockerResult<()> {
        let volume_name = self.create_auth_volume()?;
        let temp_container = "para-auth-snapshot-temp";

        // Remove any existing temp container
        let _ = Command::new("docker")
            .args(["rm", "-f", temp_container])
            .output();

        // Create a clean container with the auth volume
        let create_output = Command::new("docker")
            .args([
                "create",
                "--name",
                temp_container,
                "-v",
                &format!("{}:/root/.config", volume_name),
                "para-claude:latest",
                "sleep",
                "5",
            ])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !create_output.status.success() {
            return Err(DockerError::ContainerCreationFailed(
                String::from_utf8_lossy(&create_output.stderr).to_string(),
            ));
        }

        // Start the container briefly to ensure volume is mounted
        let start_output = Command::new("docker")
            .args(["start", temp_container])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !start_output.status.success() {
            let _ = Command::new("docker").args(["rm", temp_container]).output();
            return Err(DockerError::ContainerStartFailed(
                String::from_utf8_lossy(&start_output.stderr).to_string(),
            ));
        }

        // Wait for container to finish
        let _ = Command::new("docker")
            .args(["wait", temp_container])
            .output();

        // Commit the container as an authenticated image
        let commit_output = Command::new("docker")
            .args([
                "commit",
                "-m",
                "Para authenticated Claude image",
                temp_container,
                "para-authenticated:latest",
            ])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to commit image: {}", e)))?;

        if !commit_output.status.success() {
            let _ = Command::new("docker").args(["rm", temp_container]).output();
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to create authenticated image: {}",
                String::from_utf8_lossy(&commit_output.stderr)
            )));
        }

        // Clean up temp container
        let _ = Command::new("docker")
            .args(["rm", temp_container])
            .output();

        println!("✅ Created authenticated Docker image: para-authenticated:latest");

        Ok(())
    }

    /// Check if an authenticated image already exists
    pub fn check_authenticated_image(&self) -> DockerResult<bool> {
        let output = Command::new("docker")
            .args(["image", "inspect", "para-authenticated:latest"])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        Ok(output.status.success())
    }

    /// Clean up authentication artifacts (volumes and images)
    pub fn cleanup_auth_artifacts(&self) -> DockerResult<()> {
        let user_id = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let volume_name = format!("para-auth-claude-{}", user_id);

        // Remove the auth volume
        let volume_output = Command::new("docker")
            .args(["volume", "rm", &volume_name])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !volume_output.status.success() {
            eprintln!(
                "Warning: Failed to remove auth volume: {}",
                String::from_utf8_lossy(&volume_output.stderr)
            );
        }

        // Remove the authenticated image
        let image_output = Command::new("docker")
            .args(["rmi", "para-authenticated:latest"])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !image_output.status.success() {
            eprintln!(
                "Warning: Failed to remove authenticated image: {}",
                String::from_utf8_lossy(&image_output.stderr)
            );
        }

        println!("🧹 Cleaned up authentication artifacts");

        Ok(())
    }

    /// Run the full authentication setup flow
    pub fn run_auth_flow(&self) -> DockerResult<()> {
        // Check if authenticated image already exists
        if self.check_authenticated_image()? {
            println!("ℹ️  Authenticated image already exists. Use cleanup_auth_artifacts() to reset.");
            return Ok(());
        }

        // Run interactive authentication
        self.setup_interactive_auth()?;

        // Create authenticated snapshot
        self.create_authenticated_snapshot()?;

        Ok(())
    }
}