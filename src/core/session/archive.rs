use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::{ArchiveBranchParser, ArchivedBranchProcessor, Result, TimestampSortable};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub session_name: String,
    pub archived_at: String,
}

impl TimestampSortable for ArchiveEntry {
    fn get_timestamp(&self) -> &str {
        &self.archived_at
    }
}

pub struct ArchiveManager<'a> {
    config: &'a Config,
    git_service: &'a GitService,
}

impl<'a> ArchiveManager<'a> {
    pub fn new(config: &'a Config, git_service: &'a GitService) -> Self {
        Self {
            config,
            git_service,
        }
    }

    pub fn list_archives(&self) -> Result<Vec<ArchiveEntry>> {
        let processor = ArchivedBranchProcessor::new(self.git_service, self.config);
        processor.list_and_process(|archived_branch| self.create_archive_entry(archived_branch))
    }

    pub fn cleanup_old_archives(&self) -> Result<usize> {
        let Some(cleanup_days) = self.config.session.auto_cleanup_days else {
            return Ok(0);
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archives = self.list_archives()?;
        let mut removed_count = 0;

        for archive in archives {
            if let Ok(archived_date) = chrono::DateTime::parse_from_rfc3339(&archive.archived_at) {
                if archived_date.with_timezone(&chrono::Utc) < cutoff_date {
                    let archive_branch_name = format!(
                        "{}/archived/{}/{}",
                        self.config.get_branch_prefix(),
                        archived_date.format("%Y%m%d-%H%M%S"),
                        archive.session_name
                    );

                    if self
                        .git_service
                        .branch_manager()
                        .delete_branch(&archive_branch_name, true)
                        .is_ok()
                    {
                        removed_count += 1;
                    }
                }
            }
        }

        Ok(removed_count)
    }

    pub fn enforce_archive_limit(&self, max_archives: usize) -> Result<usize> {
        let archives = self.list_archives()?;

        if archives.len() <= max_archives {
            return Ok(0);
        }

        let mut removed_count = 0;
        let archives_to_remove = &archives[max_archives..];

        for archive in archives_to_remove {
            if let Ok(archived_date) = chrono::DateTime::parse_from_rfc3339(&archive.archived_at) {
                let archive_branch_name = format!(
                    "{}/archived/{}/{}",
                    self.config.get_branch_prefix(),
                    archived_date.format("%Y%m%d-%H%M%S"),
                    archive.session_name
                );

                if self
                    .git_service
                    .branch_manager()
                    .delete_branch(&archive_branch_name, true)
                    .is_ok()
                {
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }

    pub fn auto_cleanup(&self) -> Result<(usize, usize)> {
        let old_removed = self.cleanup_old_archives()?;
        let limit_removed = self.enforce_archive_limit(50)?; // Default limit of 50 archives
        Ok((old_removed, limit_removed))
    }

    fn create_archive_entry(&self, archived_branch: &str) -> Result<Option<ArchiveEntry>> {
        let archive_info = ArchiveBranchParser::parse_archive_branch(
            archived_branch,
            self.config.get_branch_prefix(),
        )?;

        match archive_info {
            Some(info) => {
                let archived_at = self.parse_timestamp_to_rfc3339(&info.timestamp);
                Ok(Some(ArchiveEntry {
                    session_name: info.session_name,
                    archived_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn parse_timestamp_to_rfc3339(&self, timestamp: &str) -> String {
        if timestamp.len() == 15 {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S") {
                return dt.and_utc().to_rfc3339();
            }
        }

        Utc::now().to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_archive_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let _archive_manager = ArchiveManager::new(&config, &git_service);
    }

    #[test]
    fn test_list_empty_archives() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let archives = archive_manager.list_archives().unwrap();
        assert!(archives.is_empty());
    }

    #[test]
    fn test_timestamp_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let rfc3339 = archive_manager.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(rfc3339.contains("2024-03-01T12:00:00"));
    }

    #[test]
    fn test_parse_timestamp_formats() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let valid_timestamp = archive_manager.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(valid_timestamp.contains("2024-03-01T12:00:00"));

        let invalid_timestamp = archive_manager.parse_timestamp_to_rfc3339("invalid");
        assert!(chrono::DateTime::parse_from_rfc3339(&invalid_timestamp).is_ok());
    }

    #[test]
    fn test_list_archives_sorted_by_date() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();

        for i in 1..=3 {
            let session_name = format!("session-{}", i);
            branch_manager
                .create_branch(&session_name, &initial_branch)
                .unwrap();
            git_service
                .repository()
                .checkout_branch(&initial_branch)
                .unwrap();
            branch_manager
                .move_to_archive(&session_name, config.get_branch_prefix())
                .unwrap();

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let archives = archive_manager.list_archives().unwrap();
        assert_eq!(archives.len(), 3);

        for i in 0..archives.len() - 1 {
            assert!(archives[i].archived_at >= archives[i + 1].archived_at);
        }
    }

    #[test]
    fn test_archive_limit_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();

        // Create 5 archived sessions
        for i in 1..=5 {
            let session_name = format!("limit-test-{}", i);
            branch_manager
                .create_branch(&session_name, &initial_branch)
                .unwrap();
            git_service
                .repository()
                .checkout_branch(&initial_branch)
                .unwrap();
            branch_manager
                .move_to_archive(&session_name, config.get_branch_prefix())
                .unwrap();
        }

        let archives_before = archive_manager.list_archives().unwrap();
        assert_eq!(archives_before.len(), 5);

        // Enforce limit of 3
        let removed = archive_manager.enforce_archive_limit(3).unwrap();
        assert_eq!(removed, 2);

        let archives_after = archive_manager.list_archives().unwrap();
        assert_eq!(archives_after.len(), 3);

        // Check that newest archives are kept (they're sorted by date descending)
        // Since archives are created rapidly, they may all have the same timestamp
        // Just verify we have the right count and they're valid session names
        for archive in &archives_after {
            assert!(archive.session_name.starts_with("limit-test-"));
        }
    }

    #[test]
    fn test_auto_cleanup_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.auto_cleanup_days = None; // Disable cleanup
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let removed = archive_manager.cleanup_old_archives().unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_auto_cleanup_old_archives() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.auto_cleanup_days = Some(1); // 1 day cleanup
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();

        // Create an old archived session by manually creating with old timestamp
        let old_timestamp = "20230101-120000"; // Very old date
        let old_archive_branch = format!(
            "{}/archived/{}/old-session",
            config.get_branch_prefix(),
            old_timestamp
        );

        // Create the branch first
        branch_manager
            .create_branch("temp-old", &initial_branch)
            .unwrap();
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        // Create archived format branch and delete temp
        branch_manager
            .create_branch(&old_archive_branch, "temp-old")
            .unwrap();
        branch_manager.delete_branch("temp-old", true).unwrap();

        // Create a recent archived session
        let recent_session = "recent-session";
        branch_manager
            .create_branch(recent_session, &initial_branch)
            .unwrap();
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();
        branch_manager
            .move_to_archive(recent_session, config.get_branch_prefix())
            .unwrap();

        // Verify we have 2 archives before cleanup
        let archives_before = archive_manager.list_archives().unwrap();
        assert_eq!(archives_before.len(), 2);

        // Run cleanup - should remove the old one
        let removed = archive_manager.cleanup_old_archives().unwrap();
        assert_eq!(removed, 1);

        // Verify only the recent one remains
        let archives_after = archive_manager.list_archives().unwrap();
        assert_eq!(archives_after.len(), 1);
        assert_eq!(archives_after[0].session_name, recent_session);
    }

    #[test]
    fn test_auto_cleanup_combined() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.auto_cleanup_days = Some(1); // 1 day cleanup
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();

        // Create multiple sessions - some old, some recent, some over limit
        for i in 1..=10 {
            let session_name = format!("test-session-{}", i);
            branch_manager
                .create_branch(&session_name, &initial_branch)
                .unwrap();
            git_service
                .repository()
                .checkout_branch(&initial_branch)
                .unwrap();
            branch_manager
                .move_to_archive(&session_name, config.get_branch_prefix())
                .unwrap();
        }

        let archives_before = archive_manager.list_archives().unwrap();
        assert_eq!(archives_before.len(), 10);

        // Run auto_cleanup which does both old cleanup and limit enforcement
        let (old_removed, limit_removed) = archive_manager.auto_cleanup().unwrap();

        // Should enforce limit of 50 (but we only have 10, so no limit removals)
        // and no old removals since all are recent
        assert_eq!(old_removed, 0);
        assert_eq!(limit_removed, 0);

        let archives_after = archive_manager.list_archives().unwrap();
        assert_eq!(archives_after.len(), 10); // All kept since they're recent and under limit
    }
}
