use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;

/// Manages gitignore files to ensure para directories are properly ignored
pub struct GitignoreManager {
    directory: String,
}

impl GitignoreManager {
    /// Create a new GitignoreManager for the given directory
    pub fn new(directory: &str) -> Self {
        Self {
            directory: directory.to_string(),
        }
    }

    /// Add an entry to the .gitignore file in the specified directory
    /// Returns true if the entry was added, false if it already existed
    pub fn add_entry(&self, entry: &str) -> Result<bool> {
        let gitignore_path = Path::new(&self.directory).join(".gitignore");

        // Check if entry already exists
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            if Self::is_entry_already_ignored(&content, entry) {
                return Ok(false);
            }
        }

        // Add entry to gitignore
        Self::add_entry_to_gitignore(&gitignore_path, entry)?;
        Ok(true)
    }

    /// Check if entry is already ignored in the gitignore content
    fn is_entry_already_ignored(content: &str, entry: &str) -> bool {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if line == entry {
                return true;
            }
        }
        false
    }

    /// Add entry to gitignore file
    fn add_entry_to_gitignore(gitignore_path: &Path, entry: &str) -> Result<()> {
        if gitignore_path.exists() {
            // Append to existing gitignore
            let existing_content = fs::read_to_string(gitignore_path)?;
            let new_content = if existing_content.ends_with('\n') {
                format!("{}{}\n", existing_content, entry)
            } else {
                format!("{}\n{}\n", existing_content, entry)
            };

            fs::write(gitignore_path, new_content).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to update .gitignore at {}: {}",
                    gitignore_path.display(),
                    e
                ))
            })?;
        } else {
            // Create new gitignore
            fs::write(gitignore_path, format!("{}\n", entry)).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create .gitignore at {}: {}",
                    gitignore_path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Ensure that .para directory is ignored in the repository's main .gitignore
    pub fn ensure_para_ignored_in_repository(repo_root: &Path) -> Result<()> {
        let gitignore_path = repo_root.join(".gitignore");

        // Check if .para is already ignored
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            if Self::is_para_already_ignored(&content) {
                return Ok(());
            }
        }

        // Add .para to gitignore
        Self::add_para_to_gitignore(&gitignore_path)?;
        Ok(())
    }

    /// Check if .para is already properly ignored in the gitignore content
    fn is_para_already_ignored(content: &str) -> bool {
        // Check for various forms of .para ignoring
        let patterns = [".para/", ".para", "/.para/", "/.para", ".para/*"];

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            for pattern in &patterns {
                if line == *pattern {
                    return true;
                }
            }
        }

        false
    }

    /// Add .para to gitignore file
    fn add_para_to_gitignore(gitignore_path: &Path) -> Result<()> {
        let para_entry = "\n# Para directories and state files\n# Ignore everything in .para except Dockerfile.custom\n.para/*\n!.para/Dockerfile.custom\n";

        if gitignore_path.exists() {
            // Append to existing gitignore
            let existing_content = fs::read_to_string(gitignore_path)?;
            let new_content = if existing_content.ends_with('\n') {
                format!("{}{}", existing_content, para_entry.trim_start())
            } else {
                format!("{}{}", existing_content, para_entry)
            };

            fs::write(gitignore_path, new_content).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to update .gitignore at {}: {}",
                    gitignore_path.display(),
                    e
                ))
            })?;
        } else {
            // Create new gitignore
            fs::write(gitignore_path, para_entry.trim()).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create .gitignore at {}: {}",
                    gitignore_path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Create the internal .para/.gitignore file
    pub fn create_para_internal_gitignore(para_dir: &Path) -> Result<()> {
        let gitignore_path = para_dir.join(".gitignore");

        if !gitignore_path.exists() {
            let gitignore_content =
                "# Ignore all para contents except configuration and setup scripts\n\
                 *\n\
                 !.gitignore\n\
                 !setup.sh\n\
                 !setup-docker.sh\n\
                 !setup-worktree.sh\n\
                 !Dockerfile.custom\n";
            fs::write(&gitignore_path, gitignore_content).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create .para/.gitignore file at {}: {}",
                    gitignore_path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_ensure_para_ignored_new_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        GitignoreManager::ensure_para_ignored_in_repository(repo_root).unwrap();

        let gitignore_path = repo_root.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".para/*"));
        assert!(content.contains("!.para/Dockerfile.custom"));
        assert!(content.contains("Para directories and state files"));
    }

    #[test]
    fn test_ensure_para_ignored_existing_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        let gitignore_path = repo_root.join(".gitignore");

        // Create existing gitignore
        fs::write(&gitignore_path, "*.log\ntarget/\n").unwrap();

        GitignoreManager::ensure_para_ignored_in_repository(repo_root).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("*.log"));
        assert!(content.contains("target/"));
        assert!(content.contains(".para/*"));
        assert!(content.contains("!.para/Dockerfile.custom"));
        assert!(content.contains("Para directories and state files"));
    }

    #[test]
    fn test_para_already_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        let gitignore_path = repo_root.join(".gitignore");

        // Create gitignore with .para/ already ignored
        fs::write(&gitignore_path, "*.log\n.para/\ntarget/\n").unwrap();

        GitignoreManager::ensure_para_ignored_in_repository(repo_root).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        // Should not add duplicate entry
        assert_eq!(content.matches(".para/").count(), 1);
    }

    #[test]
    fn test_is_para_already_ignored() {
        assert!(GitignoreManager::is_para_already_ignored(".para/\n"));
        assert!(GitignoreManager::is_para_already_ignored(
            "*.log\n.para/\ntarget/\n"
        ));
        assert!(GitignoreManager::is_para_already_ignored("/.para/\n"));
        assert!(GitignoreManager::is_para_already_ignored(".para\n"));
        assert!(GitignoreManager::is_para_already_ignored(".para/*\n"));

        assert!(!GitignoreManager::is_para_already_ignored(
            "*.log\ntarget/\n"
        ));
        assert!(!GitignoreManager::is_para_already_ignored("para/\n"));
        assert!(!GitignoreManager::is_para_already_ignored("some-para/\n"));
    }

    #[test]
    fn test_create_para_internal_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let para_dir = temp_dir.path().join(".para");
        fs::create_dir_all(&para_dir).unwrap();

        GitignoreManager::create_para_internal_gitignore(&para_dir).unwrap();

        let gitignore_path = para_dir.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("*"));
        assert!(content.contains("!.gitignore"));
        assert!(content.contains("!setup.sh"));
        assert!(content.contains("!setup-docker.sh"));
        assert!(content.contains("!setup-worktree.sh"));
        assert!(content.contains("!Dockerfile.custom"));
    }

    #[test]
    fn test_add_entry_to_new_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();

        let manager = GitignoreManager::new(dir_path);
        let added = manager.add_entry("build/").unwrap();

        assert!(added);
        let gitignore_path = temp_dir.path().join(".gitignore");
        assert!(gitignore_path.exists());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "build/\n");
    }

    #[test]
    fn test_add_entry_to_existing_gitignore_with_newline() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing gitignore with trailing newline
        fs::write(&gitignore_path, "*.log\ntarget/\n").unwrap();

        let manager = GitignoreManager::new(dir_path);
        let added = manager.add_entry("build/").unwrap();

        assert!(added);
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "*.log\ntarget/\nbuild/\n");
    }

    #[test]
    fn test_add_entry_to_existing_gitignore_without_newline() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing gitignore without trailing newline
        fs::write(&gitignore_path, "*.log\ntarget/").unwrap();

        let manager = GitignoreManager::new(dir_path);
        let added = manager.add_entry("build/").unwrap();

        assert!(added);
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "*.log\ntarget/\nbuild/\n");
    }

    #[test]
    fn test_add_duplicate_entry() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        fs::write(&gitignore_path, "*.log\nbuild/\ntarget/\n").unwrap();

        let manager = GitignoreManager::new(dir_path);
        let added = manager.add_entry("build/").unwrap();

        assert!(!added); // Should return false for duplicate
        let content = fs::read_to_string(&gitignore_path).unwrap();
        // Content should remain unchanged
        assert_eq!(content, "*.log\nbuild/\ntarget/\n");
    }

    #[test]
    fn test_add_entry_with_comments_and_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        let gitignore_content = "# Build artifacts\nbuild/\n\n# Dependencies\nnode_modules/\n";
        fs::write(&gitignore_path, gitignore_content).unwrap();

        let manager = GitignoreManager::new(dir_path);

        // Try to add duplicate that already exists
        let added = manager.add_entry("build/").unwrap();
        assert!(!added);

        // Add new entry
        let added = manager.add_entry("dist/").unwrap();
        assert!(added);

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("dist/"));
        assert!(content.ends_with("dist/\n"));
    }

    #[test]
    fn test_is_entry_already_ignored_edge_cases() {
        // Test with various whitespace and comment scenarios
        assert!(GitignoreManager::is_entry_already_ignored(
            "  build/  \n",
            "build/"
        ));
        assert!(GitignoreManager::is_entry_already_ignored(
            "# Some comment\nbuild/\n",
            "build/"
        ));
        assert!(GitignoreManager::is_entry_already_ignored(
            "\n\nbuild/\n\n",
            "build/"
        ));
        assert!(!GitignoreManager::is_entry_already_ignored(
            "# build/\n",
            "build/"
        ));
        assert!(!GitignoreManager::is_entry_already_ignored(
            "build\n", "build/"
        ));
    }

    #[test]
    fn test_add_entry_file_permission_error() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create a read-only gitignore file
        fs::write(&gitignore_path, "*.log\n").unwrap();
        let metadata = fs::metadata(&gitignore_path).unwrap();
        let mut perms = metadata.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&gitignore_path, perms).unwrap();

        let manager = GitignoreManager::new(dir_path);
        let result = manager.add_entry("build/");

        // Should fail with permission error
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = e.to_string();
            // Check for either update or create error message
            assert!(
                error_msg.contains("Failed to update .gitignore")
                    || error_msg.contains("Failed to create .gitignore"),
                "Error message was: {}",
                error_msg
            );
        }

        // Clean up: reset permissions
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&gitignore_path).unwrap().permissions();
            perms.set_mode(0o644); // Standard read/write permissions for owner
            fs::set_permissions(&gitignore_path, perms).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut perms = fs::metadata(&gitignore_path).unwrap().permissions();
            perms.set_readonly(false);
            fs::set_permissions(&gitignore_path, perms).unwrap();
        }
    }

    #[test]
    fn test_add_multiple_entries_sequentially() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        let manager = GitignoreManager::new(dir_path);

        // Add first entry
        assert!(manager.add_entry("build/").unwrap());

        // Add second entry
        assert!(manager.add_entry("dist/").unwrap());

        // Add third entry
        assert!(manager.add_entry("*.tmp").unwrap());

        // Try to add duplicate
        assert!(!manager.add_entry("build/").unwrap());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "build/\ndist/\n*.tmp\n");
    }
}
