use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Common session information for terminal mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub name: String,
    pub id: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub base_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_uncommitted_changes: Option<bool>,
}

/// Terminal configuration for AI commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub working_directory: PathBuf,
    pub ai_command: String,
    pub ai_args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_session_id: Option<String>,
    pub environment: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Metadata about the operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,
    pub para_version: String,
    pub ai_backend: String,
}

/// Response for successful start operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartResponse {
    pub success: bool,
    pub action: String,
    pub session: SessionInfo,
    pub terminal_config: TerminalConfig,
    pub metadata: Metadata,
}

/// Response for successful resume operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResponse {
    pub success: bool,
    pub action: String,
    pub session: SessionInfo,
    pub terminal_config: TerminalConfig,
    pub metadata: Metadata,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Response for error cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorInfo,
}

impl ErrorResponse {
    #[allow(dead_code)]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorInfo {
                code: code.into(),
                message: message.into(),
                suggestion: None,
            },
        }
    }

    #[allow(dead_code)]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error.suggestion = Some(suggestion.into());
        self
    }
}

/// Helper to build metadata
pub fn build_metadata(action: &str) -> Metadata {
    let now = Utc::now();
    let para_version = env!("CARGO_PKG_VERSION").to_string();

    match action {
        "start" => Metadata {
            created_at: Some(now),
            resumed_at: None,
            last_activity: None,
            para_version,
            ai_backend: "claude".to_string(), // Default, can be made configurable
        },
        "resume" => Metadata {
            created_at: None,
            resumed_at: Some(now),
            last_activity: None, // Will be populated from session
            para_version,
            ai_backend: "claude".to_string(),
        },
        _ => Metadata {
            created_at: None,
            resumed_at: None,
            last_activity: None,
            para_version,
            ai_backend: "claude".to_string(),
        },
    }
}

/// Validate terminal mode arguments
pub fn validate_terminal_mode_args(
    terminal: bool,
    container: bool,
    sandbox: bool,
    sandbox_no_network: bool,
) -> Result<(), String> {
    if !terminal {
        return Ok(());
    }

    if container {
        return Err("Docker containers not supported in terminal mode yet".to_string());
    }

    if sandbox {
        return Err("Sandboxing not supported in terminal mode yet".to_string());
    }

    if sandbox_no_network {
        return Err("Network sandboxing not supported in terminal mode yet".to_string());
    }

    Ok(())
}
