use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Non-blocking path normalization that doesn't follow symlinks
/// This prevents hanging on broken symlinks or unresponsive network mounts
#[cfg(test)]
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = vec![];

    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::Normal(c) => {
                components.push(c);
            }
            Component::RootDir => {
                components.clear();
                components.push(std::ffi::OsStr::new("/"));
            }
            Component::CurDir => {}
            Component::Prefix(_) => {} // Windows prefix, ignored on Unix
        }
    }

    if components.is_empty() {
        PathBuf::from(".")
    } else {
        components.iter().collect()
    }
}

/// Try to resolve a path with a timeout, falling back to normalization if it fails
pub fn safe_resolve_path(path: &Path) -> PathBuf {
    safe_resolve_path_with_timeout(path, Duration::from_secs(2))
}

/// Resolve a path with timeout protection to prevent deadlocks
pub fn safe_resolve_path_with_timeout(path: &Path, timeout: Duration) -> PathBuf {
    let path_owned = path.to_path_buf();

    // Create a channel for communication between threads
    let (tx, rx) = mpsc::channel();

    // Spawn a thread to perform the potentially blocking operations
    thread::spawn(move || {
        let result = if path_owned.exists() {
            // Try canonicalize, but if it fails (e.g., due to permission issues or broken symlinks),
            // fall back to the original path
            path_owned
                .canonicalize()
                .unwrap_or_else(|_| path_owned.clone())
        } else {
            path_owned.clone()
        };

        // Send the result back through the channel
        let _ = tx.send(result);
    });

    // Wait for the result with a timeout
    match rx.recv_timeout(timeout) {
        Ok(resolved_path) => {
            debug_log(&format!(
                "Path resolved within timeout: {}",
                resolved_path.display()
            ));
            resolved_path
        }
        Err(_) => {
            debug_log(&format!(
                "Path resolution timed out after {:?}, using original path: {}",
                timeout,
                path.display()
            ));
            // Timeout occurred, return the original path
            path.to_path_buf()
        }
    }
}

/// Debug logging helper
pub fn debug_log(message: &str) {
    if std::env::var("PARA_DEBUG").is_ok() {
        eprintln!("[PARA_DEBUG] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("/a/b/c")), PathBuf::from("/a/b/c"));
        assert_eq!(
            normalize_path(Path::new("/a/./b/c")),
            PathBuf::from("/a/b/c")
        );
        assert_eq!(
            normalize_path(Path::new("/a/b/../c")),
            PathBuf::from("/a/c")
        );
        assert_eq!(normalize_path(Path::new("a/b/c")), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path(Path::new("./a/b/c")), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path(Path::new("")), PathBuf::from("."));
    }

    #[test]
    fn test_safe_resolve_path() {
        let temp_dir = TempDir::new().unwrap();
        let existing_file = temp_dir.path().join("test.txt");
        fs::write(&existing_file, "test").unwrap();

        // Should normalize existing paths
        let resolved = safe_resolve_path(&existing_file);
        assert!(resolved.ends_with("test.txt"));

        // Should return original path for non-existent files
        let non_existent = temp_dir.path().join("non_existent.txt");
        let resolved = safe_resolve_path(&non_existent);
        assert_eq!(resolved, non_existent);
    }

    #[test]
    fn test_safe_resolve_path_with_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let existing_file = temp_dir.path().join("timeout_test.txt");
        fs::write(&existing_file, "test").unwrap();

        // Should resolve within timeout
        let resolved = safe_resolve_path_with_timeout(&existing_file, Duration::from_secs(5));
        assert!(resolved.ends_with("timeout_test.txt"));

        // Should handle non-existent files within timeout
        let non_existent = temp_dir.path().join("non_existent_timeout.txt");
        let resolved = safe_resolve_path_with_timeout(&non_existent, Duration::from_secs(1));
        assert_eq!(resolved, non_existent);
    }

    #[cfg(unix)]
    #[test]
    fn test_safe_resolve_path_with_broken_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target.txt");
        let symlink = temp_dir.path().join("broken_link");

        // Create a symlink to a non-existent target
        std::os::unix::fs::symlink(&target, &symlink).unwrap();

        // Should handle broken symlinks gracefully within timeout
        let resolved = safe_resolve_path_with_timeout(&symlink, Duration::from_secs(2));
        // Should return the symlink path itself when canonicalization fails
        assert_eq!(resolved, symlink);
    }

    #[test]
    fn test_timeout_protection() {
        // Test that very short timeout works correctly
        let temp_dir = TempDir::new().unwrap();
        let existing_file = temp_dir.path().join("short_timeout.txt");
        fs::write(&existing_file, "test").unwrap();

        // Even with a very short timeout, basic file operations should still work
        let resolved = safe_resolve_path_with_timeout(&existing_file, Duration::from_millis(100));
        assert!(resolved.ends_with("short_timeout.txt"));
    }

    #[test]
    fn test_debug_log() {
        // Test with PARA_DEBUG not set
        debug_log("This should not print");

        // Test with PARA_DEBUG set
        std::env::set_var("PARA_DEBUG", "1");
        debug_log("This should print to stderr");
        std::env::remove_var("PARA_DEBUG");
    }
}
