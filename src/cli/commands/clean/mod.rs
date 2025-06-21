pub mod coordinator;
pub mod reporter;
pub mod strategies;
pub mod validator;

pub use coordinator::CleanupCoordinator;
pub use reporter::CleanupReporter;
pub use strategies::{ArchiveCleanupStrategy, StateFileCleanupStrategy, WorktreeCleanupStrategy};
pub use validator::CleanupValidator;

use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let cleaner = SessionCleaner::new(git_service, config);
    cleaner.execute_clean(args)
}

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
        let coordinator = self.create_coordinator();
        let validator = CleanupValidator::new(self.config.clone());
        let reporter = CleanupReporter::new();

        let cleanup_plan = coordinator.analyze_cleanup(&self.config, &self.git_service)?;

        if cleanup_plan.is_empty() {
            println!("🧹 Nothing to clean - your Para environment is already tidy!");
            return Ok(());
        }

        if args.dry_run {
            validator.show_dry_run_report(&cleanup_plan);
            return Ok(());
        }

        if !args.force && !validator.confirm_cleanup(&cleanup_plan)? {
            println!("Cleanup cancelled");
            return Ok(());
        }

        let results = coordinator.perform_cleanup(cleanup_plan, &self.git_service)?;
        reporter.show_results(&results);

        Ok(())
    }

    fn create_coordinator(&self) -> CleanupCoordinator {
        CleanupCoordinator::new()
            .add_strategy(Box::new(WorktreeCleanupStrategy))
            .add_strategy(Box::new(StateFileCleanupStrategy))
            .add_strategy(Box::new(ArchiveCleanupStrategy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_cleaner_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        assert_eq!(cleaner.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_create_coordinator() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        let coordinator = cleaner.create_coordinator();
        assert_eq!(coordinator.strategy_count(), 3);
    }

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
