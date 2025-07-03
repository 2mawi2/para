use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

use super::cleanup::{
    ArchiveCleaner, BranchCleaner, CleanupPlan, CleanupResults, InteractiveHandler,
    StateFileCleaner,
};

/// Main entry point for the clean command
pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let cleaner = SessionCleaner::new(git_service, config);
    cleaner.execute_clean(args)
}

/// Orchestrates the cleanup process by coordinating different cleanup strategies
struct SessionCleaner {
    git_service: GitService,
    config: Config,
}

impl SessionCleaner {
    fn new(git_service: GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    fn execute_clean(&self, args: CleanArgs) -> Result<()> {
        let cleanup_plan = self.analyze_cleanup()?;

        if cleanup_plan.is_empty() {
            println!("🧹 Nothing to clean - your Para environment is already tidy!");
            return Ok(());
        }

        if args.dry_run {
            InteractiveHandler::show_dry_run_report(
                &cleanup_plan,
                self.config.session.auto_cleanup_days,
            );
            return Ok(());
        }

        if !args.force
            && !InteractiveHandler::confirm_cleanup(
                &cleanup_plan,
                self.config.session.auto_cleanup_days,
            )?
        {
            println!("Cleanup cancelled");
            return Ok(());
        }

        let results = self.perform_cleanup(cleanup_plan)?;
        results.show_results();

        Ok(())
    }

    /// Analyze what needs to be cleaned up
    fn analyze_cleanup(&self) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        // Find stale branches
        let branch_cleaner = BranchCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            &self.config.directories.state_dir,
        );
        plan.stale_branches = branch_cleaner.find_stale_branches()?;

        // Find orphaned state files
        let state_file_cleaner = StateFileCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            &self.config.directories.state_dir,
        );
        plan.orphaned_state_files = state_file_cleaner.find_orphaned_state_files()?;

        // Find old archives
        let archive_cleaner = ArchiveCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            self.config.session.auto_cleanup_days,
        );
        plan.old_archives = archive_cleaner.find_old_archives()?;

        Ok(plan)
    }

    /// Perform the actual cleanup operations
    fn perform_cleanup(&self, plan: CleanupPlan) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        // Clean stale branches
        let branch_cleaner = BranchCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            &self.config.directories.state_dir,
        );
        let (removed_branches, mut branch_errors) =
            branch_cleaner.remove_stale_branches(plan.stale_branches);
        results.stale_branches_removed = removed_branches;
        results.errors.append(&mut branch_errors);

        // Clean orphaned state files
        let state_file_cleaner = StateFileCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            &self.config.directories.state_dir,
        );
        let (removed_files, mut file_errors) =
            state_file_cleaner.remove_orphaned_files(plan.orphaned_state_files);
        results.orphaned_state_files_removed = removed_files;
        results.errors.append(&mut file_errors);

        // Clean old archives
        let archive_cleaner = ArchiveCleaner::new(
            &self.git_service,
            &self.config.git.branch_prefix,
            self.config.session.auto_cleanup_days,
        );
        let (removed_archives, mut archive_errors) =
            archive_cleaner.remove_old_archives(plan.old_archives);
        results.old_archives_removed = removed_archives;
        results.errors.append(&mut archive_errors);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::parser::CleanArgs;

    #[test]
    fn test_clean_args_defaults() {
        let args = CleanArgs {
            force: false,
            dry_run: false,
            backups: false,
        };

        assert!(!args.force);
        assert!(!args.dry_run);
        assert!(!args.backups);
    }
}
