use super::strategies::{CleanupItem, CleanupItemType, CleanupStrategy};
use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use dialoguer::Confirm;

pub struct CleanupCoordinator {
    git_service: GitService,
    config: Config,
}

#[derive(Debug, Default)]
pub struct CoordinatorResults {
    pub stale_branches_removed: usize,
    pub orphaned_state_files_removed: usize,
    pub old_archives_removed: usize,
    pub errors: Vec<String>,
}

impl CleanupCoordinator {
    pub fn new(git_service: GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    pub fn execute_clean(&self, args: CleanArgs) -> Result<()> {
        let cleanup_plan = self.analyze_all_strategies()?;

        if cleanup_plan.is_empty() {
            println!("🧹 Nothing to clean - your Para environment is already tidy!");
            return Ok(());
        }

        if args.dry_run {
            self.show_dry_run_report(&cleanup_plan);
            return Ok(());
        }

        if !args.force && !self.confirm_cleanup(&cleanup_plan)? {
            println!("Cleanup cancelled");
            return Ok(());
        }

        let results = self.perform_coordinated_cleanup(cleanup_plan)?;
        self.show_results(&results);

        Ok(())
    }

    fn analyze_all_strategies(&self) -> Result<Vec<CleanupItem>> {
        let mut all_items = Vec::new();

        let stale_branch_cleanup =
            super::strategies::StaleBranchCleanup::new(&self.git_service, self.config.clone());
        let orphaned_state_cleanup =
            super::strategies::OrphanedStateCleanup::new(&self.git_service, self.config.clone());
        let old_archive_cleanup =
            super::strategies::OldArchiveCleanup::new(&self.git_service, self.config.clone());

        all_items.extend(stale_branch_cleanup.analyze()?);
        all_items.extend(orphaned_state_cleanup.analyze()?);
        all_items.extend(old_archive_cleanup.analyze()?);

        Ok(all_items)
    }

    fn perform_coordinated_cleanup(&self, items: Vec<CleanupItem>) -> Result<CoordinatorResults> {
        let mut results = CoordinatorResults::default();

        let stale_branch_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::StaleBranch))
            .cloned()
            .collect();

        let orphaned_state_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OrphanedStateFile))
            .cloned()
            .collect();

        let old_archive_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OldArchive))
            .cloned()
            .collect();

        let stale_branch_cleanup =
            super::strategies::StaleBranchCleanup::new(&self.git_service, self.config.clone());
        if !stale_branch_items.is_empty() {
            let result = stale_branch_cleanup.execute(&stale_branch_items)?;
            results.stale_branches_removed += result.items_removed;
            results.errors.extend(result.errors);
        }

        let orphaned_state_cleanup =
            super::strategies::OrphanedStateCleanup::new(&self.git_service, self.config.clone());
        if !orphaned_state_items.is_empty() {
            let result = orphaned_state_cleanup.execute(&orphaned_state_items)?;
            results.orphaned_state_files_removed += result.items_removed;
            results.errors.extend(result.errors);
        }

        let old_archive_cleanup =
            super::strategies::OldArchiveCleanup::new(&self.git_service, self.config.clone());
        if !old_archive_items.is_empty() {
            let result = old_archive_cleanup.execute(&old_archive_items)?;
            results.old_archives_removed += result.items_removed;
            results.errors.extend(result.errors);
        }

        Ok(results)
    }

    fn show_dry_run_report(&self, items: &[CleanupItem]) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        let stale_branches: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::StaleBranch))
            .collect();

        let orphaned_files: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OrphanedStateFile))
            .collect();

        let old_archives: Vec<_> = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OldArchive))
            .collect();

        if !stale_branches.is_empty() {
            println!("Stale Branches ({}):", stale_branches.len());
            for item in &stale_branches {
                println!("  🌿 {}", item.identifier);
            }
            println!();
        }

        if !orphaned_files.is_empty() {
            println!("Orphaned State Files ({}):", orphaned_files.len());
            for item in &orphaned_files {
                if let Some(ref path) = item.path {
                    println!("  📝 {}", path.display());
                } else {
                    println!("  📝 {}", item.identifier);
                }
            }
            println!();
        }

        if !old_archives.is_empty() {
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!("Old Archives (older than {} days):", days);
            for item in &old_archives {
                println!("  📦 {}", item.identifier);
            }
            println!();
        }
    }

    fn confirm_cleanup(&self, items: &[CleanupItem]) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

        let stale_branches_count = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::StaleBranch))
            .count();

        let orphaned_files_count = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OrphanedStateFile))
            .count();

        let old_archives_count = items
            .iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OldArchive))
            .count();

        let total_items = stale_branches_count + orphaned_files_count + old_archives_count;

        if stale_branches_count > 0 {
            println!("  🌿 {} stale branches", stale_branches_count);
        }

        if orphaned_files_count > 0 {
            println!("  📝 {} orphaned state files", orphaned_files_count);
        }

        if old_archives_count > 0 {
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!(
                "  📦 {} archived sessions (older than {} days)",
                old_archives_count, days
            );
        }

        if total_items == 0 {
            println!("No items to clean");
            return Ok(false);
        }

        if Self::is_non_interactive() {
            return Err(crate::utils::ParaError::invalid_args(
                "Cannot perform cleanup in non-interactive mode. Use --force flag to skip confirmation prompts."
            ));
        }

        Ok(Confirm::new()
            .with_prompt("Continue with cleanup?")
            .default(false)
            .interact()
            .unwrap_or(false))
    }

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    fn show_results(&self, results: &CoordinatorResults) {
        println!("🧹 Cleanup Complete");
        println!("==================\n");

        if results.stale_branches_removed > 0 {
            println!(
                "  ✅ Removed {} stale branches",
                results.stale_branches_removed
            );
        }

        if results.orphaned_state_files_removed > 0 {
            println!(
                "  ✅ Removed {} orphaned state files",
                results.orphaned_state_files_removed
            );
        }

        if results.old_archives_removed > 0 {
            println!(
                "  ✅ Removed {} old archived sessions",
                results.old_archives_removed
            );
        }

        if !results.errors.is_empty() {
            println!("\n⚠️  Some items couldn't be cleaned:");
            for error in &results.errors {
                println!("  • {}", error);
            }
        }

        if results.stale_branches_removed == 0
            && results.orphaned_state_files_removed == 0
            && results.old_archives_removed == 0
        {
            println!("✨ Your Para environment was already clean!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_coordinator_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let _coordinator = CleanupCoordinator::new(git_service, config);

        // Coordinator stores git_service and config, strategies are created on demand
        // No specific assertions needed for constructor
    }

    #[test]
    fn test_coordinator_results_default() {
        let results = CoordinatorResults::default();
        assert_eq!(results.stale_branches_removed, 0);
        assert_eq!(results.orphaned_state_files_removed, 0);
        assert_eq!(results.old_archives_removed, 0);
        assert!(results.errors.is_empty());
    }
}
