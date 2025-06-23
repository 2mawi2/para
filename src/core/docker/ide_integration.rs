//! IDE integration module for Docker containers (MVP)
//!
//! This module provides automatic IDE-to-container connection
//! for VS Code and Cursor IDEs when working with Docker containers.

use crate::config::Config;
use crate::core::docker::session::ContainerSession;
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// MVP Docker IDE integration - automatic container connection
pub struct DockerIdeIntegration;

impl DockerIdeIntegration {
    /// Launch IDE with automatic container connection
    pub fn launch_container_ide(
        config: &Config,
        session_dir: &Path,
        container_session: &ContainerSession,
        initial_prompt: Option<&str>,
    ) -> Result<()> {
        // Save initial prompt if provided
        if let Some(prompt) = initial_prompt {
            let prompt_file = session_dir.join(".initial-prompt");
            fs::write(&prompt_file, prompt).map_err(|e| {
                ParaError::docker_error(format!("Failed to save initial prompt: {}", e))
            })?;
        }

        // Create VS Code tasks.json for auto-run
        Self::create_vscode_tasks(session_dir, initial_prompt)?;

        // Construct vscode-remote URI for direct container connection
        let container_name = format!("para-{}", container_session.session_name);
        let container_hex = Self::hex_encode_string(&container_name);
        let remote_uri = format!(
            "vscode-remote://attached-container+{}/workspace",
            container_hex
        );

        // Determine IDE command (VS Code only for MVP)
        let ide_command = if config.ide.wrapper.enabled {
            &config.ide.wrapper.command
        } else {
            &config.ide.command
        };

        // Only support VS Code for now
        if ide_command != "code" {
            return Err(ParaError::docker_error(
                "Docker container auto-connection currently only supports VS Code. Please use 'code' as your IDE command."
            ));
        }

        // Launch IDE with remote URI
        let mut cmd = Command::new(ide_command);
        cmd.arg("--folder-uri").arg(&remote_uri);
        
        // Detach the IDE process
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        match cmd.spawn() {
            Ok(_) => {
                println!("🚀 Launching VS Code connected to container: {}", container_name);
                println!("   Container: {}", container_name);
                println!("   Workspace: /workspace");
                if initial_prompt.is_some() {
                    println!();
                    println!("🤖 Claude will start automatically in the container!");
                    println!("   - VS Code will open connected to the container");
                    println!("   - Claude will run with your saved prompt");
                    println!("   - The prompt is saved to .initial-prompt");
                }
                Ok(())
            }
            Err(e) => Err(ParaError::docker_error(format!(
                "Failed to launch VS Code with container: {}",
                e
            ))),
        }
    }

    /// Create VS Code tasks.json for auto-run
    fn create_vscode_tasks(session_dir: &Path, initial_prompt: Option<&str>) -> Result<()> {
        let vscode_dir = session_dir.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| {
            ParaError::docker_error(format!("Failed to create .vscode directory: {}", e))
        })?;

        // Now that we have Claude installed in the container, we can run it directly
        let command = if initial_prompt.is_some() {
            "claude \"$(cat '/workspace/.initial-prompt')\""
        } else {
            "echo 'Para container session ready. Run: claude'"
        };

        let tasks_config = serde_json::json!({
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Start Claude Code in Container",
                    "type": "shell",
                    "command": command,
                    "runOptions": {
                        "runOn": "folderOpen"
                    },
                    "presentation": {
                        "echo": true,
                        "reveal": "always",
                        "focus": true,
                        "panel": "new"
                    }
                }
            ]
        });

        let tasks_file = vscode_dir.join("tasks.json");
        let tasks_json = serde_json::to_string_pretty(&tasks_config).map_err(|e| {
            ParaError::docker_error(format!("Failed to serialize tasks.json: {}", e))
        })?;
        
        fs::write(&tasks_file, tasks_json).map_err(|e| {
            ParaError::docker_error(format!("Failed to write tasks.json: {}", e))
        })?;

        Ok(())
    }

    /// Hex encode a string for vscode-remote URI
    fn hex_encode_string(s: &str) -> String {
        s.bytes()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }
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
    use tempfile::TempDir;

    #[test]
    fn test_hex_encode_string() {
        assert_eq!(DockerIdeIntegration::hex_encode_string("hello"), "68656c6c6f");
        assert_eq!(DockerIdeIntegration::hex_encode_string("para-test-123"), "706172612d746573742d313233");
        assert_eq!(DockerIdeIntegration::hex_encode_string(""), "");
    }

    #[test]
    fn test_create_vscode_tasks_with_prompt() {
        let temp_dir = TempDir::new().unwrap();
        
        let result = DockerIdeIntegration::create_vscode_tasks(
            temp_dir.path(),
            Some("Test prompt"),
        );
        assert!(result.is_ok());

        // Should create tasks.json that runs Claude in container
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        let content = fs::read_to_string(tasks_file).unwrap();
        assert!(content.contains("Start Claude Code in Container"));
        assert!(content.contains("claude"));
        assert!(content.contains("/workspace/.initial-prompt"));
        assert!(content.contains("folderOpen"));
    }

    #[test]
    fn test_create_vscode_tasks_without_prompt() {
        let temp_dir = TempDir::new().unwrap();
        
        let result = DockerIdeIntegration::create_vscode_tasks(
            temp_dir.path(),
            None,
        );
        assert!(result.is_ok());

        // Should still create tasks.json but with a different message
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());
        
        let content = fs::read_to_string(tasks_file).unwrap();
        assert!(content.contains("Para container session ready"));
        assert!(!content.contains("/workspace/.initial-prompt"));
    }
}
