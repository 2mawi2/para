use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub mod analyzers;
pub mod interaction;
pub mod strategies;

use analyzers::CleanupAnalyzerRegistry;
use interaction::CleanupInteraction;
use strategies::CleanupStrategy;

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
        // Create cleanup plan using analyzers
        let analyzer_registry = CleanupAnalyzerRegistry::new(
            self.git_service.clone(),
            self.config.clone()
        );
        let cleanup_plan = analyzer_registry.create_cleanup_plan()?;

        // Check if there's anything to clean
        if cleanup_plan.is_empty() {
            println!("🧹 Nothing to clean - your Para environment is already tidy!");
            return Ok(());
        }

        // Create interaction handler
        let interaction = CleanupInteraction::new(self.config.clone());

        // Execute cleanup using strategy pattern
        let strategy = CleanupStrategy::from_args(&args);
        let results = strategy.execute(cleanup_plan, &self.git_service, &interaction)?;

        // Show results (unless it was a dry run)
        if !args.dry_run {
            interaction.show_cleanup_results(&results);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_execution_dry_run() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let args = CleanArgs {
            dry_run: true,
            force: false,
            backups: false,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_execution_force() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let args = CleanArgs {
            dry_run: false,
            force: true,
            backups: false,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_cleaner_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        // Test that cleaner is created successfully
        assert!(std::ptr::addr_of!(cleaner).is_aligned());
    }
}