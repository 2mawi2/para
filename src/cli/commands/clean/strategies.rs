use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CleanupItem {
    pub item_type: CleanupItemType,
    pub identifier: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum CleanupItemType {
    StaleBranch,
    OrphanedStateFile,
    OldArchive,
}

#[derive(Debug, Default)]
pub struct CleanupResult {
    pub items_removed: usize,
    pub errors: Vec<String>,
}

pub trait CleanupStrategy {
    fn analyze(&self) -> Result<Vec<CleanupItem>>;
    fn execute(&self, items: &[CleanupItem]) -> Result<CleanupResult>;
}

pub struct StaleBranchCleanup<'a> {
    git_service: &'a GitService,
    config: Config,
}

impl<'a> StaleBranchCleanup<'a> {
    pub fn new(git_service: &'a GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }
}

impl<'a> CleanupStrategy for StaleBranchCleanup<'a> {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let mut items = Vec::new();
        let prefix = format!("{}/", self.config.git.branch_prefix);
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        let all_branches = self.git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    items.push(CleanupItem {
                        item_type: CleanupItemType::StaleBranch,
                        identifier: branch_info.name,
                        path: None,
                    });
                }
            }
        }

        Ok(items)
    }

    fn execute(&self, items: &[CleanupItem]) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for item in items {
            if matches!(item.item_type, CleanupItemType::StaleBranch) {
                match self.git_service.delete_branch(&item.identifier, true) {
                    Ok(_) => result.items_removed += 1,
                    Err(e) => result.errors.push(format!(
                        "Failed to remove branch {}: {}",
                        item.identifier, e
                    )),
                }
            }
        }

        Ok(result)
    }
}

pub struct OrphanedStateCleanup<'a> {
    git_service: &'a GitService,
    config: Config,
}

impl<'a> OrphanedStateCleanup<'a> {
    pub fn new(git_service: &'a GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    fn scan_state_directory(&self, state_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut state_files = Vec::new();

        if !state_dir.exists() {
            return Ok(state_files);
        }

        for entry in std::fs::read_dir(state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if self.is_state_file(&path) {
                state_files.push(path);
            }
        }

        Ok(state_files)
    }

    fn is_state_file(&self, path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| name.ends_with(".state"))
            .unwrap_or(false)
    }

    fn extract_session_id(&self, state_file: &std::path::Path) -> Result<String> {
        let file_name = state_file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| crate::utils::ParaError::invalid_args("Invalid state file name"))?;

        let session_id = file_name.strip_suffix(".state").ok_or_else(|| {
            crate::utils::ParaError::invalid_args("State file must end with .state")
        })?;

        Ok(session_id.to_string())
    }

    fn is_session_orphaned(&self, session_id: &str) -> Result<bool> {
        let branch_name = format!("{}/{}", self.config.git.branch_prefix, session_id);
        Ok(!self.git_service.branch_exists(&branch_name)?)
    }

    fn find_related_files(&self, state_dir: &std::path::Path, session_id: &str) -> Vec<PathBuf> {
        let mut related_files = Vec::new();

        for suffix in &[".prompt", ".launch"] {
            let related_file = state_dir.join(format!("{}{}", session_id, suffix));
            if related_file.exists() {
                related_files.push(related_file);
            }
        }

        related_files
    }
}

impl<'a> CleanupStrategy for OrphanedStateCleanup<'a> {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let state_dir = PathBuf::from(&self.config.directories.state_dir);
        let mut items = Vec::new();

        let state_files = self.scan_state_directory(&state_dir)?;

        for state_file in state_files {
            let session_id = self.extract_session_id(&state_file)?;

            if self.is_session_orphaned(&session_id)? {
                items.push(CleanupItem {
                    item_type: CleanupItemType::OrphanedStateFile,
                    identifier: session_id.clone(),
                    path: Some(state_file.clone()),
                });

                for related_file in self.find_related_files(&state_dir, &session_id) {
                    items.push(CleanupItem {
                        item_type: CleanupItemType::OrphanedStateFile,
                        identifier: format!("{} (related)", session_id),
                        path: Some(related_file),
                    });
                }
            }
        }

        Ok(items)
    }

    fn execute(&self, items: &[CleanupItem]) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for item in items {
            if matches!(item.item_type, CleanupItemType::OrphanedStateFile) {
                if let Some(ref path) = item.path {
                    match std::fs::remove_file(path) {
                        Ok(_) => result.items_removed += 1,
                        Err(e) => result.errors.push(format!(
                            "Failed to remove file {}: {}",
                            path.display(),
                            e
                        )),
                    }
                }
            }
        }

        Ok(result)
    }
}

pub struct OldArchiveCleanup<'a> {
    git_service: &'a GitService,
    config: Config,
}

impl<'a> OldArchiveCleanup<'a> {
    pub fn new(git_service: &'a GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
        branch
            .split('/')
            .nth(2)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::utils::ParaError::invalid_args(format!(
                    "Invalid archived branch format: {}",
                    branch
                ))
            })
    }

    fn parse_archive_timestamp(&self, timestamp: &str) -> Result<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S").map_err(|e| {
            crate::utils::ParaError::invalid_args(format!(
                "Invalid timestamp format '{}': {}",
                timestamp, e
            ))
        })
    }

    fn is_archive_older_than_cutoff(
        &self,
        branch: &str,
        cutoff_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        let timestamp_part = self.extract_archive_timestamp(branch)?;
        let branch_time = self.parse_archive_timestamp(&timestamp_part)?;
        Ok(branch_time.and_utc() < cutoff_date)
    }
}

impl<'a> CleanupStrategy for OldArchiveCleanup<'a> {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let cleanup_days = match self.config.session.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(Vec::new()),
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(&self.config.git.branch_prefix)?;

        let mut items = Vec::new();

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                items.push(CleanupItem {
                    item_type: CleanupItemType::OldArchive,
                    identifier: branch,
                    path: None,
                });
            }
        }

        Ok(items)
    }

    fn execute(&self, items: &[CleanupItem]) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for item in items {
            if matches!(item.item_type, CleanupItemType::OldArchive) {
                match self.git_service.delete_branch(&item.identifier, true) {
                    Ok(_) => result.items_removed += 1,
                    Err(e) => result.errors.push(format!(
                        "Failed to remove archive {}: {}",
                        item.identifier, e
                    )),
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_item_creation() {
        let item = CleanupItem {
            item_type: CleanupItemType::StaleBranch,
            identifier: "test-branch".to_string(),
            path: None,
        };

        assert!(matches!(item.item_type, CleanupItemType::StaleBranch));
        assert_eq!(item.identifier, "test-branch");
        assert!(item.path.is_none());
    }

    #[test]
    fn test_cleanup_result_default() {
        let result = CleanupResult::default();
        assert_eq!(result.items_removed, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_stale_branch_cleanup_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let _cleanup = StaleBranchCleanup::new(&git_service, config);

        // Cleanup strategy creates successfully
    }

    #[test]
    fn test_orphaned_state_cleanup_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let _cleanup = OrphanedStateCleanup::new(&git_service, config);

        // Cleanup strategy creates successfully
    }

    #[test]
    fn test_old_archive_cleanup_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let _cleanup = OldArchiveCleanup::new(&git_service, config);

        // Cleanup strategy creates successfully
    }
}
