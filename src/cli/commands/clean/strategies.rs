use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum CleanupItem {
    StaleBranch(String),
    OrphanedStateFile(PathBuf),
    OldArchive(String),
}

pub trait CleanupStrategy {
    fn find_cleanup_items(
        &self,
        config: &Config,
        git_service: &GitService,
    ) -> Result<Vec<CleanupItem>>;
    fn clean_item(&self, item: &CleanupItem, git_service: &GitService) -> Result<()>;
}

pub struct WorktreeCleanupStrategy;

impl CleanupStrategy for WorktreeCleanupStrategy {
    fn find_cleanup_items(
        &self,
        config: &Config,
        git_service: &GitService,
    ) -> Result<Vec<CleanupItem>> {
        let mut stale_branches = Vec::new();
        let prefix = format!("{}/", config.git.branch_prefix);
        let state_dir = PathBuf::from(&config.directories.state_dir);

        let all_branches = git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    stale_branches.push(CleanupItem::StaleBranch(branch_info.name));
                }
            }
        }

        Ok(stale_branches)
    }

    fn clean_item(&self, item: &CleanupItem, git_service: &GitService) -> Result<()> {
        match item {
            CleanupItem::StaleBranch(branch_name) => {
                git_service.delete_branch(branch_name, true)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "WorktreeCleanupStrategy can only clean stale branches",
            )),
        }
    }
}

pub struct StateFileCleanupStrategy;

impl CleanupStrategy for StateFileCleanupStrategy {
    fn find_cleanup_items(
        &self,
        config: &Config,
        git_service: &GitService,
    ) -> Result<Vec<CleanupItem>> {
        let state_dir = PathBuf::from(&config.directories.state_dir);

        if !state_dir.exists() {
            return Ok(Vec::new());
        }

        let mut orphaned_files = Vec::new();
        let state_files = self.scan_state_directory(&state_dir)?;

        for state_file in state_files {
            let session_id = self.extract_session_id(&state_file)?;

            if self.is_session_orphaned(&session_id, config, git_service)? {
                orphaned_files.push(CleanupItem::OrphanedStateFile(state_file.clone()));
                orphaned_files.extend(
                    self.find_related_files(&state_dir, &session_id)
                        .into_iter()
                        .map(CleanupItem::OrphanedStateFile),
                );
            }
        }

        Ok(orphaned_files)
    }

    fn clean_item(&self, item: &CleanupItem, _git_service: &GitService) -> Result<()> {
        match item {
            CleanupItem::OrphanedStateFile(file_path) => {
                fs::remove_file(file_path)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "StateFileCleanupStrategy can only clean orphaned state files",
            )),
        }
    }
}

impl StateFileCleanupStrategy {
    fn scan_state_directory(&self, state_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut state_files = Vec::new();

        for entry in fs::read_dir(state_dir)? {
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

    fn is_session_orphaned(
        &self,
        session_id: &str,
        config: &Config,
        git_service: &GitService,
    ) -> Result<bool> {
        let branch_name = format!("{}/{}", config.git.branch_prefix, session_id);
        Ok(!git_service.branch_exists(&branch_name)?)
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

pub struct ArchiveCleanupStrategy;

impl CleanupStrategy for ArchiveCleanupStrategy {
    fn find_cleanup_items(
        &self,
        config: &Config,
        git_service: &GitService,
    ) -> Result<Vec<CleanupItem>> {
        let cleanup_days = match config.session.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(Vec::new()),
        };

        let cutoff_date = Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = git_service
            .branch_manager()
            .list_archived_branches(&config.git.branch_prefix)?;

        let mut old_archives = Vec::new();

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                old_archives.push(CleanupItem::OldArchive(branch));
            }
        }

        Ok(old_archives)
    }

    fn clean_item(&self, item: &CleanupItem, git_service: &GitService) -> Result<()> {
        match item {
            CleanupItem::OldArchive(branch_name) => {
                git_service.delete_branch(branch_name, true)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "ArchiveCleanupStrategy can only clean old archives",
            )),
        }
    }
}

impl ArchiveCleanupStrategy {
    fn is_archive_older_than_cutoff(
        &self,
        branch: &str,
        cutoff_date: DateTime<Utc>,
    ) -> Result<bool> {
        let timestamp_part = self.extract_archive_timestamp(branch)?;
        let branch_time = self.parse_archive_timestamp(&timestamp_part)?;
        Ok(branch_time.and_utc() < cutoff_date)
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

    fn parse_archive_timestamp(&self, timestamp: &str) -> Result<NaiveDateTime> {
        NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S").map_err(|e| {
            crate::utils::ParaError::invalid_args(format!(
                "Invalid timestamp format '{}': {}",
                timestamp, e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_item_debug() {
        let item = CleanupItem::StaleBranch("test-branch".to_string());
        assert!(format!("{:?}", item).contains("StaleBranch"));
    }

    #[test]
    fn test_worktree_strategy_wrong_item_type() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let strategy = WorktreeCleanupStrategy;
        let item = CleanupItem::OrphanedStateFile(PathBuf::from("test.state"));

        let result = strategy.clean_item(&item, &git_service);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_timestamp_parsing() {
        let strategy = ArchiveCleanupStrategy;

        let valid_timestamp = "20230101-120000";
        let result = strategy.parse_archive_timestamp(valid_timestamp);
        assert!(result.is_ok());

        let invalid_timestamp = "invalid";
        let result = strategy.parse_archive_timestamp(invalid_timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_archive_timestamp() {
        let strategy = ArchiveCleanupStrategy;

        let valid_branch = "prefix/archived/20230101-120000/session-name";
        let result = strategy.extract_archive_timestamp(valid_branch);
        assert_eq!(result.unwrap(), "20230101-120000");

        let invalid_branch = "prefix/archived";
        let result = strategy.extract_archive_timestamp(invalid_branch);
        assert!(result.is_err());
    }
}
