use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;

/// Template content for CLAUDE.local.md
const CLAUDE_LOCAL_TEMPLATE: &str = include_str!("../../templates/claude_local.md");

/// Create CLAUDE.local.md file with instructions for AI agents
pub fn create_claude_local_md(session_path: &Path, session_name: &str) -> Result<()> {
    // Ensure the session path exists
    if !session_path.exists() {
        return Err(ParaError::fs_error(format!(
            "Session path does not exist: {}",
            session_path.display()
        )));
    }

    let claude_local_path = session_path.join("CLAUDE.local.md");

    // Replace placeholder with actual session name
    let content = CLAUDE_LOCAL_TEMPLATE.replace("{session_name}", session_name);

    // Write the file (overwrite if exists)
    fs::write(&claude_local_path, content)
        .map_err(|e| ParaError::fs_error(format!("Failed to write CLAUDE.local.md: {}", e)))?;

    Ok(())
}
