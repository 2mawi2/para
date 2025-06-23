//! IDE integration module for Docker containers (MVP)
//!
//! This module provides basic devcontainer configuration generation
//! for VS Code and Cursor IDEs when working with Docker containers.

use crate::core::docker::session::ContainerSession;
use crate::utils::{ParaError, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

/// MVP Docker IDE integration - generates basic devcontainer config
pub struct DockerIdeIntegration;

impl DockerIdeIntegration {
    /// Generate basic devcontainer configuration for a container session
    pub fn generate_devcontainer_config(
        session_dir: &Path,
        container_session: &ContainerSession,
        initial_prompt: Option<&str>,
    ) -> Result<()> {
        // Create .devcontainer directory
        let devcontainer_dir = session_dir.join(".devcontainer");
        fs::create_dir_all(&devcontainer_dir).map_err(|e| {
            ParaError::docker_error(format!("Failed to create .devcontainer directory: {}", e))
        })?;

        // Save initial prompt if provided
        if let Some(prompt) = initial_prompt {
            let prompt_file = session_dir.join(".initial-prompt");
            fs::write(&prompt_file, prompt).map_err(|e| {
                ParaError::docker_error(format!("Failed to save initial prompt: {}", e))
            })?;
        }

        // Generate basic devcontainer.json
        let container_name = format!("para-{}", container_session.session_name);
        let devcontainer_config = DevContainerConfig {
            name: format!("Para: {}", container_session.session_name),
            docker_compose_file: None,
            service: None,
            workspace_folder: "/workspace".to_string(),
            remote_user: "vscode".to_string(),
            post_attach_command: initial_prompt
                .map(|p| format!("echo 'Initial prompt saved in .initial-prompt: {}'", p)),
            extensions: vec!["ms-vscode.remote-explorer".to_string()],
            settings: serde_json::json!({
                "terminal.integrated.defaultProfile.linux": "bash",
            }),
            forward_ports: vec![],
            remote_env: serde_json::json!({
                "PARA_SESSION": container_session.session_name.clone(),
                "PARA_CONTAINER": container_session.container_id.clone(),
            }),
        };

        let devcontainer_json =
            serde_json::to_string_pretty(&devcontainer_config).map_err(|e| {
                ParaError::docker_error(format!("Failed to serialize devcontainer.json: {}", e))
            })?;

        let devcontainer_file = devcontainer_dir.join("devcontainer.json");
        fs::write(&devcontainer_file, devcontainer_json).map_err(|e| {
            ParaError::docker_error(format!("Failed to write devcontainer.json: {}", e))
        })?;

        println!(
            "✅ Generated .devcontainer/devcontainer.json for container: {}",
            container_name
        );

        Ok(())
    }
}

/// Basic Dev Container configuration structure (MVP)
#[derive(Debug, Serialize)]
struct DevContainerConfig {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dockerComposeFile")]
    docker_compose_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<String>,
    #[serde(rename = "workspaceFolder")]
    workspace_folder: String,
    #[serde(rename = "remoteUser")]
    remote_user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "postAttachCommand")]
    post_attach_command: Option<String>,
    extensions: Vec<String>,
    settings: serde_json::Value,
    #[serde(rename = "forwardPorts")]
    forward_ports: Vec<u16>,
    #[serde(rename = "remoteEnv")]
    remote_env: serde_json::Value,
}

// TODO: Connect to CLI in next phase
// The following features are planned for future implementation:
// - Auto-detection of IDE type (VS Code, Cursor, Claude)
// - Generation of .vscode/tasks.json for auto-run
// - Remote container URI generation
// - Manual connection instructions
// - IDE-specific command building

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_container_session(name: &str) -> ContainerSession {
        ContainerSession::new(
            "test-container-123".to_string(),
            name.to_string(),
            "ubuntu:latest".to_string(),
            PathBuf::from("/workspace"),
        )
    }

    #[test]
    fn test_generate_devcontainer_config_basic() {
        let temp_dir = TempDir::new().unwrap();
        let container_session = create_test_container_session("test-session");

        let result = DockerIdeIntegration::generate_devcontainer_config(
            temp_dir.path(),
            &container_session,
            None,
        );
        assert!(result.is_ok());

        let devcontainer_file = temp_dir.path().join(".devcontainer/devcontainer.json");
        assert!(devcontainer_file.exists());

        let content = fs::read_to_string(devcontainer_file).unwrap();
        assert!(content.contains("Para: test-session"));
        assert!(content.contains("PARA_SESSION"));
        assert!(content.contains("test-container-123"));
    }

    #[test]
    fn test_generate_devcontainer_config_with_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let container_session = create_test_container_session("test-session");

        let result = DockerIdeIntegration::generate_devcontainer_config(
            temp_dir.path(),
            &container_session,
            Some("Test initial prompt"),
        );
        assert!(result.is_ok());

        // Check prompt file was created
        let prompt_file = temp_dir.path().join(".initial-prompt");
        assert!(prompt_file.exists());
        let prompt_content = fs::read_to_string(prompt_file).unwrap();
        assert_eq!(prompt_content, "Test initial prompt");

        // Check devcontainer has postAttachCommand
        let devcontainer_file = temp_dir.path().join(".devcontainer/devcontainer.json");
        let content = fs::read_to_string(devcontainer_file).unwrap();
        assert!(content.contains("postAttachCommand"));
        assert!(content.contains("Initial prompt saved"));
    }
}
