use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;

/// Handles cleanup of old archived sessions based on configured retention period
pub struct ArchiveCleaner<'a> {
    git_service: &'a GitService,
    branch_prefix: &'a str,
    auto_cleanup_days: Option<u32>,
}

impl<'a> ArchiveCleaner<'a> {
    pub fn new(
        git_service: &'a GitService,
        branch_prefix: &'a str,
        auto_cleanup_days: Option<u32>,
    ) -> Self {
        Self {
            git_service,
            branch_prefix,
            auto_cleanup_days,
        }
    }

    /// Find old archived sessions that are older than the configured retention period
    pub fn find_old_archives(&self) -> Result<Vec<String>> {
        let cleanup_days = match self.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(Vec::new()),
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(self.branch_prefix)?;

        let mut old_archives = Vec::new();

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                old_archives.push(branch);
            }
        }

        Ok(old_archives)
    }

    /// Remove old archived sessions and return the count of successfully removed archives
    pub fn remove_old_archives(&self, old_archives: Vec<String>) -> (usize, Vec<String>) {
        let mut removed_count = 0;
        let mut errors = Vec::new();

        for archive_branch in old_archives {
            match self.git_service.delete_branch(&archive_branch, true) {
                Ok(_) => removed_count += 1,
                Err(e) => errors.push(format!(
                    "Failed to remove archive {}: {}",
                    archive_branch, e
                )),
            }
        }

        (removed_count, errors)
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

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
        // Extract timestamp from branch name: prefix/archived/TIMESTAMP/name
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
}
