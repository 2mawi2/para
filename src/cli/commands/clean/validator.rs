use super::coordinator::CleanupPlan;
use crate::config::Config;
use crate::utils::Result;
use dialoguer::Confirm;

pub struct CleanupValidator {
    config: Config,
}

impl CleanupValidator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    pub fn show_dry_run_report(&self, plan: &CleanupPlan) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        self.show_stale_branches(plan);
        self.show_orphaned_state_files(plan);
        self.show_old_archives(plan);
    }

    pub fn confirm_cleanup(&self, plan: &CleanupPlan) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

        let total_items = self.show_cleanup_summary(plan);

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

    fn show_stale_branches(&self, plan: &CleanupPlan) {
        let stale_branches = plan.stale_branches();
        if !stale_branches.is_empty() {
            println!("Stale Branches ({}):", stale_branches.len());
            for branch in stale_branches {
                println!("  🌿 {}", branch);
            }
            println!();
        }
    }

    fn show_orphaned_state_files(&self, plan: &CleanupPlan) {
        let orphaned_files = plan.orphaned_state_files();
        if !orphaned_files.is_empty() {
            println!("Orphaned State Files ({}):", orphaned_files.len());
            for file in orphaned_files {
                println!("  📝 {}", file.display());
            }
            println!();
        }
    }

    fn show_old_archives(&self, plan: &CleanupPlan) {
        let old_archives = plan.old_archives();
        if !old_archives.is_empty() {
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!("Old Archives (older than {} days):", days);
            for archive in old_archives {
                println!("  📦 {}", archive);
            }
            println!();
        }
    }

    fn show_cleanup_summary(&self, plan: &CleanupPlan) -> usize {
        let mut total_items = 0;

        let stale_branches = plan.stale_branches();
        if !stale_branches.is_empty() {
            println!("  🌿 {} stale branches", stale_branches.len());
            total_items += stale_branches.len();
        }

        let orphaned_files = plan.orphaned_state_files();
        if !orphaned_files.is_empty() {
            println!("  📝 {} orphaned state files", orphaned_files.len());
            total_items += orphaned_files.len();
        }

        let old_archives = plan.old_archives();
        if !old_archives.is_empty() {
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!(
                "  📦 {} archived sessions (older than {} days)",
                old_archives.len(),
                days
            );
            total_items += old_archives.len();
        }

        total_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::clean::strategies::CleanupItem;
    use crate::test_utils::test_helpers::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_validator_creation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config();
        let validator = CleanupValidator::new(config);

        // Just verify it was created successfully
        assert_eq!(validator.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_show_cleanup_summary_empty() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config();
        let validator = CleanupValidator::new(config);
        let plan = CleanupPlan::new();

        let total = validator.show_cleanup_summary(&plan);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_show_cleanup_summary_with_items() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config();
        let validator = CleanupValidator::new(config);

        let mut plan = CleanupPlan::new();
        plan.items
            .push(CleanupItem::StaleBranch("branch1".to_string()));
        plan.items
            .push(CleanupItem::OrphanedStateFile(PathBuf::from("file1.state")));

        let total = validator.show_cleanup_summary(&plan);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_is_non_interactive() {
        // This test will depend on the environment, but we can test the function exists
        let _result = CleanupValidator::is_non_interactive();
        // No assertion needed, just ensuring the function can be called
    }
}
