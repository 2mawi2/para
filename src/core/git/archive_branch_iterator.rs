use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

/// Trait for types that have a timestamp for sorting
pub trait HasTimestamp {
    fn timestamp(&self) -> &str;
}

/// Shared iterator for processing archived branches
/// Eliminates duplication between archive.rs and recovery.rs
pub struct ArchiveBranchIterator<'a> {
    git_service: &'a GitService,
    config: &'a Config,
}

impl<'a> ArchiveBranchIterator<'a> {
    pub fn new(git_service: &'a GitService, config: &'a Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    /// List archived entries using a parser function
    /// The parser function takes a branch name and returns an optional parsed entry
    pub fn list_archived_entries<T, F>(&self, parser: F) -> Result<Vec<T>>
    where
        F: Fn(&str) -> Result<Option<T>>,
        T: HasTimestamp,
    {
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(self.config.get_branch_prefix())?;

        let mut entries = Vec::new();

        for archived_branch in archived_branches {
            if let Some(entry) = parser(&archived_branch)? {
                entries.push(entry);
            }
        }

        // Sort by timestamp descending (newest first)
        entries.sort_by(|a, b| b.timestamp().cmp(a.timestamp()));
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone)]
    struct TestEntry {
        name: String,
        timestamp: String,
    }

    impl HasTimestamp for TestEntry {
        fn timestamp(&self) -> &str {
            &self.timestamp
        }
    }

    #[test]
    fn test_archive_branch_iterator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let _iterator = ArchiveBranchIterator::new(&git_service, &config);
    }

    #[test]
    fn test_list_empty_archived_entries() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let iterator = ArchiveBranchIterator::new(&git_service, &config);

        let parser = |_branch: &str| -> Result<Option<TestEntry>> { Ok(None) };

        let entries: Vec<TestEntry> = iterator.list_archived_entries(parser).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_list_archived_entries_with_parser() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let iterator = ArchiveBranchIterator::new(&git_service, &config);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();

        // Create some archived branches
        for i in 1..=3 {
            let session_name = format!("test-session-{i}");
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

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Define a simple parser that extracts session name from archived branch
        let parser = |branch: &str| -> Result<Option<TestEntry>> {
            if branch.contains("/archived/") {
                let parts: Vec<&str> = branch.split('/').collect();
                if parts.len() >= 4 {
                    let session_name = parts[3];
                    let timestamp_part = parts[2];
                    return Ok(Some(TestEntry {
                        name: session_name.to_string(),
                        timestamp: timestamp_part.to_string(),
                    }));
                }
            }
            Ok(None)
        };

        let entries: Vec<TestEntry> = iterator.list_archived_entries(parser).unwrap();
        assert_eq!(entries.len(), 3);

        // Verify sorting (newest first)
        for i in 0..entries.len() - 1 {
            assert!(entries[i].timestamp >= entries[i + 1].timestamp);
        }
    }

    #[test]
    fn test_sorting_by_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let iterator = ArchiveBranchIterator::new(&git_service, &config);

        // Create archive-style branches with timestamp in the name
        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        let archive_prefix = config.get_branch_prefix();
        let archive_branches = [
            format!("{archive_prefix}/archived/20240101-120000/test1"),
            format!("{archive_prefix}/archived/20240102-120000/test2"),
            format!("{archive_prefix}/archived/20240103-120000/test3"),
        ];

        for branch in &archive_branches {
            branch_manager
                .create_branch(branch, &initial_branch)
                .unwrap();
        }

        let parser = |branch: &str| -> Result<Option<TestEntry>> {
            if branch.contains("/archived/") {
                let parts: Vec<&str> = branch.split('/').collect();
                if parts.len() >= 4 {
                    let session_name = parts[3];
                    let timestamp_part = parts[2];
                    return Ok(Some(TestEntry {
                        name: session_name.to_string(),
                        timestamp: timestamp_part.to_string(),
                    }));
                }
            }
            Ok(None)
        };

        let entries: Vec<TestEntry> = iterator.list_archived_entries(parser).unwrap();
        assert_eq!(entries.len(), 3);

        // Should be sorted by timestamp descending (newest first)
        assert_eq!(entries[0].name, "test3"); // 20240103 - newest
        assert_eq!(entries[1].name, "test2"); // 20240102
        assert_eq!(entries[2].name, "test1"); // 20240101 - oldest
    }
}
