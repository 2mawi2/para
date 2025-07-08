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
    let project_dir = get_claude_project_dir(worktree_path)?;
    let project_dir = match project_dir {
        Some(dir) => dir,
        None => return Ok(None),
    };

    let session_files = get_valid_session_files(&project_dir)?;
    if session_files.is_empty() {
        return Ok(None);
    }

    // Find the most recent session that has actual content (not empty/broken)
    for session_file in session_files.iter().rev() {
        if let Some(session) = try_extract_session(session_file)? {
            return Ok(Some(session));
        }
    }

    // If no session with content > 1000 bytes, fall back to the most recent regardless of size
    if let Some(latest_session) = session_files.last() {
        if let Some(session) = try_extract_session_fallback(latest_session)? {
            return Ok(Some(session));
        }
    }

    Ok(None)
}

/// Get the Claude project directory for a given worktree path
fn get_claude_project_dir(worktree_path: &Path) -> Result<Option<PathBuf>> {
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

    Ok(Some(project_dir))
}

/// Get valid session files sorted by modification time
fn get_valid_session_files(project_dir: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut session_files: Vec<_> = fs::read_dir(project_dir)?
        .filter_map(|entry| entry.ok())
        .filter(is_valid_session_file)
        .collect();

    // Sort by modification time to get the most recent
    session_files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    Ok(session_files)
}

/// Check if a directory entry is a valid session file
fn is_valid_session_file(entry: &fs::DirEntry) -> bool {
    let path = entry.path();
    // Check it's a .jsonl file with a valid UUID-like name
    path.extension().map(|ext| ext == "jsonl").unwrap_or(false)
        && path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| !s.is_empty() && s.len() >= 10)
            .unwrap_or(false)
}

/// Try to extract a session from a file if it has meaningful content
fn try_extract_session(session_file: &fs::DirEntry) -> Result<Option<ClaudeSession>> {
    let path = session_file.path();
    let session_id = match get_session_id_from_path(&path) {
        Some(id) => id,
        None => return Ok(None),
    };

    if has_meaningful_content(session_file)? {
        return Ok(Some(ClaudeSession {
            id: session_id.to_string(),
        }));
    }

    Ok(None)
}

/// Try to extract a session from a file regardless of content size (fallback)
fn try_extract_session_fallback(session_file: &fs::DirEntry) -> Result<Option<ClaudeSession>> {
    let path = session_file.path();
    let session_id = match get_session_id_from_path(&path) {
        Some(id) => id,
        None => return Ok(None),
    };

    Ok(Some(ClaudeSession {
        id: session_id.to_string(),
    }))
}

/// Extract session ID from a file path
fn get_session_id_from_path(path: &Path) -> Option<&str> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty() && s.len() >= 10)
}

/// Check if a session file has meaningful content (> 1000 bytes indicates a real session)
fn has_meaningful_content(session_file: &fs::DirEntry) -> Result<bool> {
    match session_file.metadata() {
        Ok(metadata) => Ok(metadata.len() > 1000),
        Err(_) => Ok(false),
    }
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

        // Save original value and set test value
        let original_home = std::env::var("PARA_TEST_HOME").ok();
        std::env::set_var("PARA_TEST_HOME", temp_dir.path());

        let result = find_claude_session(&worktree_path).unwrap();

        // Clean up test environment
        std::env::remove_var("PARA_TEST_HOME");

        // Restore original PARA_TEST_HOME if it existed
        if let Some(original) = original_home {
            std::env::set_var("PARA_TEST_HOME", original);
        }

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

        // Create a session file with meaningful content
        let session_id = "12345678-1234-1234-1234-123456789012";
        let session_file = project_dir.join(format!("{session_id}.jsonl"));
        // Create content > 1000 bytes to simulate a real session
        let content = "x".repeat(1001);
        fs::write(&session_file, content).unwrap();

        // Use thread-local environment variable isolation
        let original_home = std::env::var("PARA_TEST_HOME").ok();

        // Set the test environment variable with isolation
        std::env::set_var("PARA_TEST_HOME", home_dir);

        // Verify environment variable is set correctly before testing
        assert_eq!(
            std::env::var("PARA_TEST_HOME").unwrap(),
            home_dir.to_string_lossy()
        );

        let result = find_claude_session(worktree_path).unwrap();

        // Clean up test environment immediately after use
        std::env::remove_var("PARA_TEST_HOME");

        // Restore original PARA_TEST_HOME if it existed
        if let Some(original) = original_home {
            std::env::set_var("PARA_TEST_HOME", original);
        }

        assert!(
            result.is_some(),
            "find_claude_session should find the session file we created"
        );
        let session = result.unwrap();
        assert_eq!(session.id, session_id);
    }
}
