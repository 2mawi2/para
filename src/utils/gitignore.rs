use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;

/// Manages gitignore files to ensure para directories are properly ignored
pub struct GitignoreManager;

impl GitignoreManager {
    /// Create a new GitignoreManager for the given directory
    pub fn new(_directory: &str) -> Self {
        Self
    }

    /// Add an entry to the .gitignore file in the specified directory
    pub fn add_entry(&self, entry: &str) -> Result<()> {
        let gitignore_path = Path::new(".gitignore");

        // Check if entry already exists
        if gitignore_path.exists() {
            let content = fs::read_to_string(gitignore_path)?;
            if Self::is_entry_already_ignored(&content, entry) {
                return Ok(());
            }
        }

        // Add entry to gitignore
        Self::add_entry_to_gitignore(gitignore_path, entry)?;
        Ok(())
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
        let patterns = [".para/", ".para", "/.para/", "/.para"];

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
        let para_entry = "\n# Para directories and state files\n.para/\n";

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
                "# Ignore all para contents except configuration\n*\n!.gitignore\n";
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

    #[test]
    fn test_ensure_para_ignored_new_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        GitignoreManager::ensure_para_ignored_in_repository(repo_root).unwrap();

        let gitignore_path = repo_root.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".para/"));
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
        assert!(content.contains(".para/"));
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
    }
}
