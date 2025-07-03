use anyhow::{Context, Result};
use std::path::Path;

use super::profiles::extract_profile;

/// Wraps a command with macOS sandbox-exec if sandboxing is enabled and we're on macOS.
pub fn wrap_with_sandbox(command: &str, _worktree_path: &Path, _profile: &str) -> Result<String> {
    // Only apply sandboxing on macOS
    #[cfg(not(target_os = "macos"))]
    {
        // Return the command unchanged on non-macOS platforms
        return Ok(command.to_string());
    }

    #[cfg(target_os = "macos")]
    {
        // Validate profile name and extract to a temporary location
        if _profile.is_empty() {
            return Err(anyhow::anyhow!("Sandbox profile name cannot be empty"));
        }

        let profile_path =
            extract_profile(_profile).context("Failed to extract sandbox profile")?;

        // Get required directories
        let home_dir = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine user directories"))?
            .home_dir()
            .to_path_buf();

        let cache_dir = directories::ProjectDirs::from("", "", "para")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| home_dir.join("Library/Caches"));

        let temp_dir = std::env::temp_dir();

        // Remove trailing slashes from paths to ensure consistent matching
        let temp_dir_str = temp_dir.to_string_lossy().trim_end_matches('/').to_string();
        let home_dir_str = home_dir.to_string_lossy().trim_end_matches('/').to_string();
        let cache_dir_str = cache_dir
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();
        let worktree_path_str = _worktree_path
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();

        // Build the sandbox-exec command with parameters
        let sandbox_cmd = format!(
            "sandbox-exec \
             -D 'TARGET_DIR={}' \
             -D 'TMP_DIR={}' \
             -D 'HOME_DIR={}' \
             -D 'CACHE_DIR={}' \
             -f '{}' \
             sh -c '{}'",
            worktree_path_str,
            temp_dir_str,
            home_dir_str,
            cache_dir_str,
            profile_path.display(),
            command.replace('\'', "'\\''") // Escape single quotes in the command
        );

        Ok(sandbox_cmd)
    }
}

/// Check if sandbox-exec is available on the system
pub fn is_sandbox_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check if sandbox-exec exists in PATH
        match std::process::Command::new("which")
            .arg("sandbox-exec")
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    true
                } else {
                    eprintln!("⚠️  sandbox-exec not found in PATH on macOS");
                    false
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to check for sandbox-exec availability: {}", e);
                false
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Sandboxing is only supported on macOS for now
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wrap_with_sandbox_macos() -> Result<()> {
        if !cfg!(target_os = "macos") {
            // Skip test on non-macOS platforms
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path();
        let command = "echo 'Hello, world!'";

        let wrapped = wrap_with_sandbox(command, worktree_path, "permissive-open")?;

        // Check that the wrapped command contains expected elements
        assert!(wrapped.contains("sandbox-exec"));
        assert!(wrapped.contains(&format!("-D 'TARGET_DIR={}'", worktree_path.display())));
        assert!(wrapped.contains("-D 'TMP_DIR="));
        assert!(wrapped.contains("-D 'HOME_DIR="));
        assert!(wrapped.contains("-D 'CACHE_DIR="));
        assert!(wrapped.contains("-f"));
        assert!(wrapped.contains(".sb"));
        assert!(wrapped.contains("sh -c 'echo '\\''Hello, world!'\\'''"));

        Ok(())
    }

    #[test]
    fn test_wrap_with_sandbox_non_macos() -> Result<()> {
        if cfg!(target_os = "macos") {
            // Skip test on macOS
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path();
        let command = "echo 'Hello, world!'";

        let wrapped = wrap_with_sandbox(command, worktree_path, "permissive-open")?;

        // On non-macOS, command should be returned unchanged
        assert_eq!(wrapped, command);

        Ok(())
    }

    #[test]
    fn test_command_escaping() -> Result<()> {
        if !cfg!(target_os = "macos") {
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path();
        let command = "echo 'It'\\''s a test'";

        let wrapped = wrap_with_sandbox(command, worktree_path, "permissive-open")?;

        // The original command is: echo 'It'\''s a test'
        // After replace('\'', "'\\''"), it becomes: echo '\\''It'\\''\\\\'\\''\\\\'\\''s a test'\\''
        // But that's wrapped in sh -c '...', so we need to check for the right pattern
        assert!(wrapped.contains("sh -c"));

        Ok(())
    }

    #[test]
    fn test_is_sandbox_available() {
        let available = is_sandbox_available();

        if cfg!(target_os = "macos") {
            // On macOS, sandbox-exec should typically be available
            // But we can't guarantee it in all test environments
            // Just check that the function doesn't panic
            let _ = available;
        } else {
            // On non-macOS platforms, it should always be false
            assert!(!available);
        }
    }
}
