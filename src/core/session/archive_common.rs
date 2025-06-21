use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::{ArchiveBranchParser, Result};
use chrono::Utc;

/// Common operations for working with archived session branches
pub struct ArchiveBranchOperations<'a> {
    config: &'a Config,
    git_service: &'a GitService,
}

#[derive(Debug, Clone)]
pub struct ArchiveBranchInfo {
    pub full_branch_name: String,
    pub session_name: String,
    pub timestamp: String,
}

impl<'a> ArchiveBranchOperations<'a> {
    pub fn new(config: &'a Config, git_service: &'a GitService) -> Self {
        Self {
            config,
            git_service,
        }
    }

    /// List all archived branches for the configured branch prefix
    pub fn list_archived_branches(&self) -> Result<Vec<String>> {
        self.git_service
            .branch_manager()
            .list_archived_branches(self.config.get_branch_prefix())
    }

    /// Parse an archived branch name into its components
    pub fn parse_archive_branch(&self, archived_branch: &str) -> Result<Option<ArchiveBranchInfo>> {
        let archive_info = ArchiveBranchParser::parse_archive_branch(
            archived_branch,
            self.config.get_branch_prefix(),
        )?;

        match archive_info {
            Some(info) => Ok(Some(ArchiveBranchInfo {
                full_branch_name: info.full_branch_name,
                session_name: info.session_name,
                timestamp: info.timestamp,
            })),
            None => Ok(None),
        }
    }

    /// Create an archive branch name from session name and timestamp
    pub fn create_archive_branch_name(&self, session_name: &str, timestamp: &str) -> String {
        format!(
            "{}/archived/{}/{}",
            self.config.get_branch_prefix(),
            timestamp,
            session_name
        )
    }

    /// Parse timestamp from YYYYMMDD-HHMMSS format to RFC3339
    pub fn parse_timestamp_to_rfc3339(&self, timestamp: &str) -> String {
        if timestamp.len() == 15 {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S") {
                return dt.and_utc().to_rfc3339();
            }
        }

        Utc::now().to_rfc3339()
    }

    /// Validate that an archived branch actually exists and has a valid commit
    pub fn validate_archived_branch(&self, archived_branch: &str) -> Result<bool> {
        let branch_manager = self.git_service.branch_manager();
        match branch_manager.get_branch_commit(archived_branch) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_archive_operations_creation() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let _operations = ArchiveBranchOperations::new(&config, &git_service);
    }

    #[test]
    fn test_list_empty_archived_branches() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let operations = ArchiveBranchOperations::new(&config, &git_service);

        let branches = operations.list_archived_branches().unwrap();
        assert!(branches.is_empty());
    }

    #[test]
    fn test_parse_archive_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let operations = ArchiveBranchOperations::new(&config, &git_service);

        let archived_branch = "test/archived/20240301-120000/my-session";

        // Create the branch first for parsing to work
        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch(archived_branch, &initial_branch)
            .unwrap();

        let info = operations
            .parse_archive_branch(archived_branch)
            .unwrap()
            .unwrap();

        assert_eq!(info.full_branch_name, archived_branch);
        assert_eq!(info.session_name, "my-session");
        assert_eq!(info.timestamp, "20240301-120000");
    }

    #[test]
    fn test_create_archive_branch_name() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let operations = ArchiveBranchOperations::new(&config, &git_service);

        let branch_name = operations.create_archive_branch_name("my-session", "20240301-120000");
        assert_eq!(branch_name, "test/archived/20240301-120000/my-session");
    }

    #[test]
    fn test_timestamp_to_rfc3339() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let operations = ArchiveBranchOperations::new(&config, &git_service);

        let rfc3339 = operations.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(rfc3339.contains("2024-03-01T12:00:00"));

        // Test invalid timestamp falls back to current time
        let invalid_rfc3339 = operations.parse_timestamp_to_rfc3339("invalid");
        assert!(chrono::DateTime::parse_from_rfc3339(&invalid_rfc3339).is_ok());
    }

    #[test]
    fn test_validate_archived_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let operations = ArchiveBranchOperations::new(&config, &git_service);

        // Test with non-existent branch
        let invalid_result = operations.validate_archived_branch("non-existent-branch");
        assert!(invalid_result.is_ok());
        assert!(!invalid_result.unwrap());

        // Test with existing branch
        let archived_branch = "test/archived/20240301-120000/my-session";
        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch(archived_branch, &initial_branch)
            .unwrap();

        let valid_result = operations.validate_archived_branch(archived_branch);
        assert!(valid_result.is_ok());
        assert!(valid_result.unwrap());
    }
}
