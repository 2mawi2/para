pub mod analyzers;
pub mod interaction;
pub mod strategies;

pub use analyzers::{CleanupAnalyzerRegistry, CleanupPlan};
pub use interaction::CleanupInteraction;
pub use strategies::{CleanupResults, CleanupStrategy};

use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;

    // Create analyzer registry and generate cleanup plan
    let analyzers = CleanupAnalyzerRegistry::new(&git_service, &config);
    let plan = analyzers.create_cleanup_plan()?;

    if plan.is_empty() {
        println!("🧹 Nothing to clean - your Para environment is already tidy!");
        return Ok(());
    }

    // Handle different execution strategies
    let strategy = CleanupStrategy::from_args(&args);
    let interaction = CleanupInteraction::new(config.clone());

    match strategy {
        CleanupStrategy::DryRun => {
            interaction.show_cleanup_preview(&plan);
            Ok(())
        }
        CleanupStrategy::Interactive => {
            if interaction.confirm_cleanup(&plan)? {
                let results = strategy.execute(plan, &git_service, &config)?;
                interaction.show_cleanup_results(&results);
            } else {
                println!("Cleanup cancelled");
            }
            Ok(())
        }
        CleanupStrategy::Force => {
            let results = strategy.execute(plan, &git_service, &config)?;
            interaction.show_cleanup_results(&results);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

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

    #[test]
    fn test_cleanup_strategy_from_args() {
        let dry_run_args = CleanArgs {
            force: false,
            dry_run: true,
            backups: false,
        };
        matches!(CleanupStrategy::from_args(&dry_run_args), CleanupStrategy::DryRun);

        let force_args = CleanArgs {
            force: true,
            dry_run: false,
            backups: false,
        };
        matches!(CleanupStrategy::from_args(&force_args), CleanupStrategy::Force);

        let interactive_args = CleanArgs {
            force: false,
            dry_run: false,
            backups: false,
        };
        matches!(CleanupStrategy::from_args(&interactive_args), CleanupStrategy::Interactive);
    }

    #[test]
    fn test_empty_cleanup_plan() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let analyzers = CleanupAnalyzerRegistry::new(&git_service, &config);
        let plan = analyzers.create_cleanup_plan().unwrap();

        assert!(plan.is_empty());
    }
}