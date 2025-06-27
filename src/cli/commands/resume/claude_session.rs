use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Claude session information
#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub id: String,
}

/// Find Claude session ID for a given worktree path
pub fn find_claude_session(worktree_path: &Path) -> Result<Option<ClaudeSession>> {
    let home_dir = if cfg!(test) {
        // In tests, allow overriding the home directory with PARA_TEST_HOME
        std::env::var("PARA_TEST_HOME")
            .or_else(|_| std::env::var("HOME"))
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| ParaError::config_error("Could not find home directory"))?
    } else {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| ParaError::config_error("Could not find home directory"))?
    };
    let claude_dir = PathBuf::from(home_dir).join(".claude");

    if !claude_dir.exists() {
        return Ok(None);
    }

    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Ok(None);
    }

    // Sanitize the worktree path to match Claude's format
    let sanitized_path = sanitize_path_for_claude(worktree_path);
    let project_dir = projects_dir.join(&sanitized_path);

    if !project_dir.exists() {
        return Ok(None);
    }

    // Find the most recent session file
    let mut session_files: Vec<_> = fs::read_dir(&project_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            // Check it's a .jsonl file with a valid UUID-like name
            path.extension().map(|ext| ext == "jsonl").unwrap_or(false)
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| !s.is_empty() && s.len() >= 10)
                    .unwrap_or(false)
        })
        .collect();

    // Sort by modification time to get the most recent
    session_files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    // Find the most recent session that has actual content (not empty/broken)
    for session_file in session_files.iter().rev() {
        if let Some(session_id) = session_file.path().file_stem().and_then(|s| s.to_str()) {
            // Validate session ID format (should be a UUID-like string)
            if !session_id.is_empty() && session_id.len() >= 10 {
                // Check if the session file has meaningful content (> 1000 bytes indicates a real session)
                if let Ok(metadata) = session_file.metadata() {
                    if metadata.len() > 1000 {
                        return Ok(Some(ClaudeSession {
                            id: session_id.to_string(),
                        }));
                    }
                }
            }
        }
    }

    // If no session with content > 1000 bytes, fall back to the most recent regardless of size
    if let Some(latest_session) = session_files.last() {
        if let Some(session_id) = latest_session.path().file_stem().and_then(|s| s.to_str()) {
            if !session_id.is_empty() && session_id.len() >= 10 {
                return Ok(Some(ClaudeSession {
                    id: session_id.to_string(),
                }));
            }
        }
    }

    Ok(None)
}

/// Sanitize a path to match Claude's directory naming convention
fn sanitize_path_for_claude(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_path_for_claude() {
        let path = Path::new("/Users/test/Documents/project");
        let sanitized = sanitize_path_for_claude(path);
        assert_eq!(sanitized, "-Users-test-Documents-project");

        let path2 = Path::new("/home/user/code/my-app");
        let sanitized2 = sanitize_path_for_claude(path2);
        assert_eq!(sanitized2, "-home-user-code-my-app");

        // Test dot replacement
        let path3 = Path::new("/Users/john.doe/Documents/my.project");
        let sanitized3 = sanitize_path_for_claude(path3);
        assert_eq!(sanitized3, "-Users-john-doe-Documents-my-project");
    }

    #[test]
    fn test_find_claude_session_no_claude_dir() {
        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().join("worktree");

        // Mock home directory without .claude
        std::env::set_var("PARA_TEST_HOME", temp_dir.path());
        let result = find_claude_session(&worktree_path).unwrap();

        // Clean up test environment
        std::env::remove_var("PARA_TEST_HOME");

        assert!(result.is_none());
    }

    #[test]
    fn test_find_claude_session_with_session() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();
        let claude_dir = home_dir.join(".claude");
        let projects_dir = claude_dir.join("projects");

        // Create Claude directory structure
        fs::create_dir_all(&projects_dir).unwrap();

        let worktree_path = Path::new("/test/worktree");
        let sanitized_path = sanitize_path_for_claude(worktree_path);
        let project_dir = projects_dir.join(&sanitized_path);
        fs::create_dir_all(&project_dir).unwrap();

        // Create a session file
        let session_id = "12345678-1234-1234-1234-123456789012";
        let session_file = project_dir.join(format!("{}.jsonl", session_id));
        fs::write(&session_file, "{}").unwrap();

        // Mock home directory
        std::env::set_var("PARA_TEST_HOME", home_dir);
        let result = find_claude_session(worktree_path).unwrap();

        // Clean up test environment
        std::env::remove_var("PARA_TEST_HOME");

        assert!(result.is_some());
        let session = result.unwrap();
        assert_eq!(session.id, session_id);
    }
}
