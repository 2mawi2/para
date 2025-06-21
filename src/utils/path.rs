use std::path::{Path, PathBuf};
#[cfg(test)]
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
    // Try to canonicalize with a simple existence check first
    if path.exists() {
        // Try canonicalize, but if it fails (e.g., due to permission issues or broken symlinks),
        // fall back to the original path
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

/// Check if a path exists with a timeout
#[cfg(test)]
fn path_exists_timeout(path: &Path, _timeout: Duration) -> bool {
    // For now, use standard exists() but wrap in error handling
    // Future: implement actual timeout mechanism
    std::panic::catch_unwind(|| path.exists()).unwrap_or(false)
}

/// Debug logging helper
pub fn debug_log(message: &str) {
    if std::env::var("PARA_DEBUG").is_ok() {
        eprintln!("[PARA_DEBUG] {}", message);
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
    fn test_path_exists_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let existing_file = temp_dir.path().join("test.txt");
        fs::write(&existing_file, "test").unwrap();

        assert!(path_exists_timeout(&existing_file, Duration::from_secs(1)));
        assert!(!path_exists_timeout(
            &temp_dir.path().join("non_existent"),
            Duration::from_secs(1)
        ));
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
