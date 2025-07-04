use crate::config::Config;
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Template content for CLAUDE.local.md
const CLAUDE_LOCAL_TEMPLATE: &str = include_str!("../../templates/claude_local.md");

/// Create CLAUDE.local.md file with instructions for AI agents
pub fn create_claude_local_md(session_path: &Path, session_name: &str) -> Result<()> {
    if !session_path.exists() {
        return Err(ParaError::fs_error(format!(
            "Session path does not exist: {}",
            session_path.display()
        )));
    }

    let claude_local_path = session_path.join("CLAUDE.local.md");

    let content = CLAUDE_LOCAL_TEMPLATE.replace("{session_name}", session_name);

    fs::write(&claude_local_path, content)
        .map_err(|e| ParaError::fs_error(format!("Failed to write CLAUDE.local.md: {e}")))?;

    Ok(())
}

/// Determine which setup script to use based on priority order
pub fn get_setup_script_path(
    cli_arg: &Option<PathBuf>,
    repo_root: &Path,
    config: &Config,
    is_docker: bool,
) -> Option<PathBuf> {
    // 1. CLI argument has highest priority
    if let Some(path) = cli_arg {
        if path.exists() {
            return Some(path.clone());
        } else {
            eprintln!("Warning: Setup script '{}' not found", path.display());
            return None;
        }
    }

    // 2. Check for environment-specific default scripts
    if is_docker {
        let docker_script = repo_root.join(".para/setup-docker.sh");
        if docker_script.exists() {
            return Some(docker_script);
        }
    } else {
        let worktree_script = repo_root.join(".para/setup-worktree.sh");
        if worktree_script.exists() {
            return Some(worktree_script);
        }
    }

    // 3. Check for generic default .para/setup.sh
    let default_script = repo_root.join(".para/setup.sh");
    if default_script.exists() {
        return Some(default_script);
    }

    // 4. Check config for setup script path
    // For Docker, check docker.setup_script first, then fall back to general setup_script
    if is_docker {
        if let Some(docker_config) = &config.docker {
            if let Some(script_path) = &docker_config.setup_script {
                let config_script = if Path::new(script_path).is_absolute() {
                    PathBuf::from(script_path)
                } else {
                    repo_root.join(script_path)
                };
                if config_script.exists() {
                    return Some(config_script);
                } else {
                    eprintln!(
                        "Warning: Docker config setup script '{}' not found",
                        config_script.display()
                    );
                }
            }
        }
    }

    // Check general setup_script in config
    if let Some(script_path) = &config.setup_script {
        let config_script = if Path::new(script_path).is_absolute() {
            PathBuf::from(script_path)
        } else {
            repo_root.join(script_path)
        };
        if config_script.exists() {
            return Some(config_script);
        } else {
            eprintln!(
                "Warning: Config setup script '{}' not found",
                config_script.display()
            );
        }
    }

    None
}

/// Run a setup script for a regular worktree session
pub fn run_worktree_setup_script(
    script_path: &Path,
    session_name: &str,
    worktree_path: &Path,
) -> Result<()> {
    use std::process::Command;

    println!("🔧 Running setup script: {}", script_path.display());

    eprintln!("⚠️  Warning: Setup scripts run with your full user permissions!");
    eprintln!("   Only run scripts from trusted sources.");
    eprintln!("   Script: {}", script_path.display());

    let mut cmd = Command::new("bash");
    cmd.arg(script_path);
    cmd.current_dir(worktree_path);

    // Set environment variables
    cmd.env("PARA_WORKSPACE", worktree_path);
    cmd.env("PARA_SESSION", session_name);

    let status = cmd
        .status()
        .map_err(|e| ParaError::ide_error(format!("Failed to execute setup script: {e}")))?;

    if !status.success() {
        return Err(ParaError::ide_error(format!(
            "Setup script failed with exit code: {}",
            status.code().unwrap_or(-1)
        )));
    }

    println!("✅ Setup script completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_setup_script_path_cli_priority() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create scripts
        let cli_script = repo_root.join("cli-setup.sh");
        let default_script = repo_root.join(".para/setup.sh");
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&cli_script, "#!/bin/bash\necho cli").unwrap();
        fs::write(&default_script, "#!/bin/bash\necho default").unwrap();

        let config = crate::test_utils::test_helpers::create_test_config();

        // CLI argument should take priority
        let result = get_setup_script_path(&Some(cli_script.clone()), repo_root, &config, false);
        assert_eq!(result, Some(cli_script));
    }

    #[test]
    fn test_get_setup_script_path_default_location() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create default script
        let default_script = repo_root.join(".para/setup.sh");
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&default_script, "#!/bin/bash\necho default").unwrap();

        let config = crate::test_utils::test_helpers::create_test_config();

        // Should find default script when no CLI argument
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, Some(default_script));
    }

    #[test]
    fn test_get_setup_script_path_config_general() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create config script
        let config_script = repo_root.join("scripts/setup.sh");
        fs::create_dir_all(repo_root.join("scripts")).unwrap();
        fs::write(&config_script, "#!/bin/bash\necho config").unwrap();

        let mut config = crate::test_utils::test_helpers::create_test_config();
        config.setup_script = Some("scripts/setup.sh".to_string());

        // Should find config script when no CLI argument or default
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, Some(config_script));
    }

    #[test]
    fn test_get_setup_script_path_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        let config = crate::test_utils::test_helpers::create_test_config();

        // Should return None when no scripts exist
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, None);

        // Should return None for nonexistent CLI script
        let nonexistent = repo_root.join("nonexistent.sh");
        let result = get_setup_script_path(&Some(nonexistent), repo_root, &config, false);
        assert_eq!(result, None);
    }
}
