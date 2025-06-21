use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

/// Trait for items that can be sorted by timestamp
pub trait TimestampSortable {
    fn get_timestamp(&self) -> &str;
}

/// Shared processor for handling archived branch operations
pub struct ArchivedBranchProcessor<'a> {
    git_service: &'a GitService,
    config: &'a Config,
}

impl<'a> ArchivedBranchProcessor<'a> {
    pub fn new(git_service: &'a GitService, config: &'a Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    /// List and process archived branches using the provided parser function
    /// Returns items sorted by timestamp in descending order (newest first)
    pub fn list_and_process<T, F>(&self, parser: F) -> Result<Vec<T>>
    where
        F: Fn(&str) -> Result<Option<T>>,
        T: TimestampSortable,
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

        entries.sort_by(|a, b| b.get_timestamp().cmp(a.get_timestamp()));
        Ok(entries)
    }
}
