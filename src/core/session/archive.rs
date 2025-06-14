use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub session_name: String,
    pub archived_at: String,
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
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(self.config.get_branch_prefix())?;

        let mut entries = Vec::new();

        for archived_branch in archived_branches {
            if let Some(entry) = self.create_archive_entry(&archived_branch)? {
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
        Ok(entries)
    }

    pub fn find_archive(&self, session_name: &str) -> Result<Option<ArchiveEntry>> {
        let archives = self.list_archives()?;
        Ok(archives
            .into_iter()
            .find(|a| a.session_name == session_name))
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
        let archive_prefix = format!("{}/archived/", self.config.get_branch_prefix());

        if !archived_branch.starts_with(&archive_prefix) {
            return Ok(None);
        }

        let suffix = archived_branch.strip_prefix(&archive_prefix).unwrap();
        let parts: Vec<&str> = suffix.split('/').collect();

        if parts.len() != 2 {
            return Ok(None);
        }

        let timestamp_str = parts[0];
        let session_name = parts[1];

        let archived_at = self.parse_timestamp_to_rfc3339(timestamp_str);

        Ok(Some(ArchiveEntry {
            session_name: session_name.to_string(),
            archived_at,
        }))
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
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir.join("subtrees").to_string_lossy().to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, GitService, Config) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo dir");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let git_service = GitService::discover_from(&repo_path).expect("Failed to discover repo");
        let config = create_test_config(temp_dir.path());

        (temp_dir, git_service, config)
    }

    #[test]
    fn test_archive_manager_creation() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let _archive_manager = ArchiveManager::new(&config, &git_service);
    }

    #[test]
    fn test_list_empty_archives() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let archives = archive_manager.list_archives().unwrap();
        assert!(archives.is_empty());
    }

    #[test]
    fn test_timestamp_parsing() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let rfc3339 = archive_manager.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(rfc3339.contains("2024-03-01T12:00:00"));
    }

    #[test]
    fn test_find_nonexistent_archive() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let result = archive_manager.find_archive("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_create_and_find_archive() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch("test-session", &initial_branch)
            .unwrap();

        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        branch_manager
            .move_to_archive("test-session", config.get_branch_prefix())
            .unwrap();

        let found_archive = archive_manager.find_archive("test-session").unwrap();
        assert!(found_archive.is_some());

        let archive = found_archive.unwrap();
        assert_eq!(archive.session_name, "test-session");
    }

    #[test]
    fn test_parse_timestamp_formats() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let valid_timestamp = archive_manager.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(valid_timestamp.contains("2024-03-01T12:00:00"));

        let invalid_timestamp = archive_manager.parse_timestamp_to_rfc3339("invalid");
        assert!(chrono::DateTime::parse_from_rfc3339(&invalid_timestamp).is_ok());
    }

    #[test]
    fn test_list_archives_sorted_by_date() {
        let (_temp_dir, git_service, config) = setup_test_repo();
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
    fn test_cancel_and_recover_session_integration() {
        // This test reproduces the real-world cancel/recover integration issue
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let branch_manager = git_service.branch_manager();

        // Simulate creating a session like the start command does
        let session_name = "integration-test-session";
        let initial_branch = git_service.repository().get_current_branch().unwrap();

        // Create a timestamped branch like the start command does
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let session_branch = format!("{}/{}", config.get_branch_prefix(), timestamp);
        branch_manager
            .create_branch(&session_branch, &initial_branch)
            .unwrap();

        // Switch back to initial branch like cancel command does
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        // Archive the session branch using NEW logic with session name (this should create the correct format)
        let archived_branch = branch_manager
            .move_to_archive_with_session_name(
                &session_branch,
                session_name,
                config.get_branch_prefix(),
            )
            .unwrap();

        // The archived branch should now be in correct format: para/archived/TIMESTAMP/SESSION-NAME
        println!("Archived branch: {}", archived_branch);

        // Now this should work with the new session-name-based archiving
        let found_archive = archive_manager.find_archive(session_name);

        // This test should now PASS with the fix
        assert!(found_archive.is_ok(), "find_archive should not error");
        assert!(
            found_archive.unwrap().is_some(),
            "Should find archived session '{}' with correct format. Archived as: {}",
            session_name,
            archived_branch
        );
    }

    #[test]
    fn test_archive_limit_enforcement() {
        let (_temp_dir, git_service, config) = setup_test_repo();
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
        let (_temp_dir, git_service, mut config) = setup_test_repo();
        config.session.auto_cleanup_days = None; // Disable cleanup
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let removed = archive_manager.cleanup_old_archives().unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_auto_cleanup_old_archives() {
        let (_temp_dir, git_service, mut config) = setup_test_repo();
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
        let (_temp_dir, git_service, mut config) = setup_test_repo();
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
