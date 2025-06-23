//! IDE integration module for Docker containers
//!
//! This module provides functionality to automatically connect IDEs
//! to Docker containers, supporting VS Code, Cursor, and Claude Code.

use crate::config::Config;
use crate::core::docker::session::ContainerSession;
use crate::utils::{ParaError, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

/// IDE integration manager for Docker containers
pub struct DockerIdeIntegration {
    config: Config,
}

impl DockerIdeIntegration {
    /// Create a new Docker IDE integration manager
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Set up IDE integration for a container session
    pub fn setup_ide_integration(
        &self,
        session_dir: &Path,
        container_session: &ContainerSession,
        initial_prompt: Option<&str>,
    ) -> Result<()> {
        // Detect the IDE from configuration
        let ide_type = self.detect_ide_type()?;

        // Save initial prompt if provided
        if let Some(prompt) = initial_prompt {
            self.save_initial_prompt(session_dir, prompt)?;
        }

        // Generate appropriate configuration based on IDE type
        match ide_type {
            IdeType::VsCode | IdeType::Cursor => {
                self.generate_vscode_config(session_dir, container_session)?;
                self.generate_devcontainer_config(session_dir, container_session, initial_prompt)?;
            }
            IdeType::Claude => {
                // Claude Code uses VS Code as wrapper, so same config
                self.generate_vscode_config(session_dir, container_session)?;
                self.generate_devcontainer_config(session_dir, container_session, initial_prompt)?;
            }
            IdeType::Unknown => {
                // Generate generic configs that work with most IDEs
                self.generate_vscode_config(session_dir, container_session)?;
                self.generate_devcontainer_config(session_dir, container_session, initial_prompt)?;
            }
        }

        // Generate connection instructions
        self.generate_connection_instructions(session_dir, container_session, ide_type)?;

        Ok(())
    }

    /// Detect the IDE type from configuration
    fn detect_ide_type(&self) -> Result<IdeType> {
        let ide_name = &self.config.ide.name.to_lowercase();
        let wrapper_name = &self.config.ide.wrapper.name.to_lowercase();

        Ok(match (ide_name.as_str(), wrapper_name.as_str()) {
            ("claude", _) => IdeType::Claude,
            (_, "cursor") | ("cursor", _) => IdeType::Cursor,
            (_, "code") | ("code", _) => IdeType::VsCode,
            _ => IdeType::Unknown,
        })
    }

    /// Save the initial prompt to a file
    fn save_initial_prompt(&self, session_dir: &Path, prompt: &str) -> Result<()> {
        let prompt_file = session_dir.join(".initial-prompt");
        fs::write(&prompt_file, prompt).map_err(|e| {
            ParaError::docker_error(format!("Failed to save initial prompt: {}", e))
        })?;
        Ok(())
    }

    /// Generate VS Code tasks.json for auto-connection
    fn generate_vscode_config(
        &self,
        session_dir: &Path,
        container_session: &ContainerSession,
    ) -> Result<()> {
        let vscode_dir = session_dir.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| {
            ParaError::docker_error(format!("Failed to create .vscode directory: {}", e))
        })?;

        // Generate tasks.json with auto-run task
        let task_config = VsCodeTasks {
            version: "2.0.0".to_string(),
            tasks: vec![VsCodeTask {
                label: format!("Connect to {} container", container_session.session_name),
                type_field: "shell".to_string(),
                command: self.build_connection_command(container_session)?,
                group: TaskGroup {
                    kind: "build".to_string(),
                    is_default: true,
                },
                presentation: TaskPresentation {
                    echo: true,
                    reveal: "always".to_string(),
                    focus: true,
                    panel: "new".to_string(),
                    show_reuse_message: false,
                    clear: false,
                },
                run_options: RunOptions {
                    run_on: "folderOpen".to_string(),
                },
            }],
        };

        let tasks_json = serde_json::to_string_pretty(&task_config).map_err(|e| {
            ParaError::docker_error(format!("Failed to serialize tasks.json: {}", e))
        })?;

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, tasks_json).map_err(|e| {
            ParaError::docker_error(format!("Failed to write tasks.json: {}", e))
        })?;

        Ok(())
    }

    /// Generate .devcontainer/devcontainer.json for VS Code Dev Containers support
    fn generate_devcontainer_config(
        &self,
        session_dir: &Path,
        container_session: &ContainerSession,
        initial_prompt: Option<&str>,
    ) -> Result<()> {
        let devcontainer_dir = session_dir.join(".devcontainer");
        fs::create_dir_all(&devcontainer_dir).map_err(|e| {
            ParaError::docker_error(format!("Failed to create .devcontainer directory: {}", e))
        })?;

        let post_attach_command = if let Some(prompt) = initial_prompt {
            // Build command to run the saved prompt
            match self.detect_ide_type()? {
                IdeType::Claude => {
                    format!("claude --dangerously-skip-permissions -p \"{}\"", prompt)
                }
                _ => {
                    // For other IDEs, just echo the prompt
                    format!("echo 'Initial prompt: {}'", prompt)
                }
            }
        } else {
            "echo 'Para container session started'".to_string()
        };

        let devcontainer_config = DevContainerConfig {
            name: format!("Para: {}", container_session.session_name),
            docker_compose_file: None,
            service: None,
            workspace_folder: "/workspace".to_string(),
            remote_user: "vscode".to_string(),
            post_attach_command: Some(post_attach_command),
            extensions: vec![
                "ms-vscode.remote-explorer".to_string(),
                "github.copilot".to_string(),
            ],
            settings: serde_json::json!({
                "terminal.integrated.defaultProfile.linux": "bash",
                "terminal.integrated.profiles.linux": {
                    "bash": {
                        "path": "/bin/bash"
                    }
                }
            }),
            forward_ports: vec![],
            remote_env: serde_json::json!({
                "PARA_SESSION": container_session.session_name.clone(),
                "PARA_CONTAINER": container_session.container_id.clone(),
            }),
        };

        let devcontainer_json = serde_json::to_string_pretty(&devcontainer_config).map_err(|e| {
            ParaError::docker_error(format!("Failed to serialize devcontainer.json: {}", e))
        })?;

        let devcontainer_file = devcontainer_dir.join("devcontainer.json");
        fs::write(&devcontainer_file, devcontainer_json).map_err(|e| {
            ParaError::docker_error(format!("Failed to write devcontainer.json: {}", e))
        })?;

        Ok(())
    }

    /// Build the connection command for the IDE
    fn build_connection_command(&self, container_session: &ContainerSession) -> Result<String> {
        let container_name = container_session.get_container_name();
        
        // For now, return a simple docker exec command
        // In a real implementation, this would launch the IDE with proper remote connection
        Ok(format!(
            "docker exec -it {} /bin/bash",
            container_name
        ))
    }

    /// Generate the remote-container URI for direct connection
    pub fn generate_container_uri(&self, container_session: &ContainerSession) -> String {
        let container_name = container_session.get_container_name();
        format!(
            "vscode-remote://attached-container+{}/workspace",
            hex::encode(container_name)
        )
    }

    /// Generate connection instructions for manual connection
    fn generate_connection_instructions(
        &self,
        session_dir: &Path,
        container_session: &ContainerSession,
        ide_type: IdeType,
    ) -> Result<()> {
        let instructions_file = session_dir.join("CONNECT.md");
        let container_name = container_session.get_container_name();
        let container_uri = self.generate_container_uri(container_session);

        let instructions = format!(
            r#"# Para Docker Container Connection Instructions

## Container Information
- **Session Name**: {}
- **Container Name**: {}
- **Container ID**: {}
- **Status**: {}

## Automatic Connection (Recommended)

### VS Code / Cursor
Open this directory in VS Code or Cursor. The container connection will be established automatically.

```bash
code {}
# or
cursor {}
```

### Direct Container URI
You can also connect directly using this URI:
```
{}
```

## Manual Connection

### 1. Using VS Code Remote-Containers Extension
1. Install the "Remote - Containers" extension
2. Open Command Palette (Cmd/Ctrl + Shift + P)
3. Run "Remote-Containers: Attach to Running Container..."
4. Select "{}"

### 2. Using Docker CLI
```bash
docker exec -it {} /bin/bash
```

### 3. Using Para CLI
```bash
para resume {}
```

## Initial Prompt
{}"#,
            container_session.session_name,
            container_name,
            &container_session.container_id,
            format!("{:?}", container_session.status),
            session_dir.display(),
            session_dir.display(),
            container_uri,
            container_name,
            container_name,
            container_session.session_name,
            if session_dir.join(".initial-prompt").exists() {
                "The initial prompt is saved in `.initial-prompt` file."
            } else {
                "No initial prompt was provided."
            }
        );

        fs::write(&instructions_file, instructions).map_err(|e| {
            ParaError::docker_error(format!("Failed to write connection instructions: {}", e))
        })?;

        // Print key instructions to console
        match ide_type {
            IdeType::VsCode | IdeType::Cursor => {
                println!("\n📋 Container connection configured!");
                println!("   Run `{} {}` to connect automatically", 
                    self.config.ide.wrapper.command, 
                    session_dir.display()
                );
            }
            IdeType::Claude => {
                println!("\n📋 Container connection configured!");
                println!("   Claude Code will connect automatically when opened in VS Code wrapper");
            }
            IdeType::Unknown => {
                println!("\n📋 Container connection instructions saved to:");
                println!("   {}", instructions_file.display());
            }
        }

        Ok(())
    }
}

/// IDE type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
enum IdeType {
    VsCode,
    Cursor,
    Claude,
    Unknown,
}

/// VS Code tasks.json structure
#[derive(Debug, Serialize)]
struct VsCodeTasks {
    version: String,
    tasks: Vec<VsCodeTask>,
}

#[derive(Debug, Serialize)]
struct VsCodeTask {
    label: String,
    #[serde(rename = "type")]
    type_field: String,
    command: String,
    group: TaskGroup,
    presentation: TaskPresentation,
    #[serde(rename = "runOptions")]
    run_options: RunOptions,
}

#[derive(Debug, Serialize)]
struct TaskGroup {
    kind: String,
    #[serde(rename = "isDefault")]
    is_default: bool,
}

#[derive(Debug, Serialize)]
struct TaskPresentation {
    echo: bool,
    reveal: String,
    focus: bool,
    panel: String,
    #[serde(rename = "showReuseMessage")]
    show_reuse_message: bool,
    clear: bool,
}

#[derive(Debug, Serialize)]
struct RunOptions {
    #[serde(rename = "runOn")]
    run_on: String,
}

/// Dev Container configuration structure
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use crate::core::docker::session::{ContainerStatus, ResourceLimits};
    use tempfile::TempDir;

    fn create_test_config(ide_name: &str, wrapper_name: &str) -> Config {
        Config {
            ide: IdeConfig {
                name: ide_name.to_string(),
                command: format!("{}", ide_name.to_lowercase()),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: true,
                    name: wrapper_name.to_string(),
                    command: wrapper_name.to_lowercase(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: ".para_state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn create_test_container_session(name: &str) -> ContainerSession {
        use std::collections::HashMap;
        use std::path::PathBuf;
        
        ContainerSession {
            container_id: "abc123".to_string(),
            session_name: name.to_string(),
            status: ContainerStatus::Running,
            created_at: chrono::Utc::now(),
            started_at: Some(chrono::Utc::now()),
            stopped_at: None,
            image: "ubuntu:latest".to_string(),
            volumes: vec![],
            ports: vec![],
            environment: HashMap::new(),
            working_dir: PathBuf::from("/workspace"),
            network_mode: "bridge".to_string(),
            hostname: format!("para-{}", name),
            resource_limits: ResourceLimits::default(),
            labels: HashMap::new(),
            health_check: None,
        }
    }

    #[test]
    fn test_ide_type_detection() {
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);
        assert_eq!(integration.detect_ide_type().unwrap(), IdeType::Claude);

        let config = create_test_config("any", "cursor");
        let integration = DockerIdeIntegration::new(config);
        assert_eq!(integration.detect_ide_type().unwrap(), IdeType::Cursor);

        let config = create_test_config("any", "code");
        let integration = DockerIdeIntegration::new(config);
        assert_eq!(integration.detect_ide_type().unwrap(), IdeType::VsCode);

        let config = create_test_config("unknown", "unknown");
        let integration = DockerIdeIntegration::new(config);
        assert_eq!(integration.detect_ide_type().unwrap(), IdeType::Unknown);
    }

    #[test]
    fn test_save_initial_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);

        let prompt = "Create a new feature for user authentication";
        integration.save_initial_prompt(temp_dir.path(), prompt).unwrap();

        let prompt_file = temp_dir.path().join(".initial-prompt");
        assert!(prompt_file.exists());
        let saved_prompt = fs::read_to_string(prompt_file).unwrap();
        assert_eq!(saved_prompt, prompt);
    }

    #[test]
    fn test_generate_vscode_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);
        let container_session = create_test_container_session("test-session");

        integration.generate_vscode_config(temp_dir.path(), &container_session).unwrap();

        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        let content = fs::read_to_string(tasks_file).unwrap();
        assert!(content.contains("Connect to test-session container"));
        assert!(content.contains("folderOpen"));
        assert!(content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_generate_devcontainer_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);
        let container_session = create_test_container_session("test-session");

        integration.generate_devcontainer_config(
            temp_dir.path(), 
            &container_session,
            Some("Test prompt")
        ).unwrap();

        let devcontainer_file = temp_dir.path().join(".devcontainer/devcontainer.json");
        assert!(devcontainer_file.exists());

        let content = fs::read_to_string(devcontainer_file).unwrap();
        assert!(content.contains("Para: test-session"));
        assert!(content.contains("postAttachCommand"));
        assert!(content.contains("claude --dangerously-skip-permissions"));
    }

    #[test]
    fn test_generate_container_uri() {
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);
        let container_session = create_test_container_session("test-session");

        let uri = integration.generate_container_uri(&container_session);
        assert!(uri.starts_with("vscode-remote://attached-container+"));
        assert!(uri.ends_with("/workspace"));
    }

    #[test]
    fn test_full_ide_integration_setup() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "code");
        let integration = DockerIdeIntegration::new(config);
        let container_session = create_test_container_session("test-session");

        integration.setup_ide_integration(
            temp_dir.path(),
            &container_session,
            Some("Initial test prompt")
        ).unwrap();

        // Check all generated files
        assert!(temp_dir.path().join(".initial-prompt").exists());
        assert!(temp_dir.path().join(".vscode/tasks.json").exists());
        assert!(temp_dir.path().join(".devcontainer/devcontainer.json").exists());
        assert!(temp_dir.path().join("CONNECT.md").exists());

        // Verify connection instructions
        let instructions = fs::read_to_string(temp_dir.path().join("CONNECT.md")).unwrap();
        assert!(instructions.contains("test-session"));
        assert!(instructions.contains("para-test-session"));
        assert!(instructions.contains("Initial Prompt"));
    }
}