use super::analyzers::CleanupPlan;
use super::strategies::CleanupResults;
use crate::config::Config;
use crate::utils::Result;
use dialoguer::Confirm;

pub struct CleanupInteraction {
    config: Config,
}

impl CleanupInteraction {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn confirm_cleanup(&self, plan: &CleanupPlan) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

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

    pub fn show_cleanup_preview(&self, plan: &CleanupPlan) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        let stale_branches = plan.stale_branches();
        if !stale_branches.is_empty() {
            println!("Stale Branches ({}):", stale_branches.len());
            for branch in stale_branches {
                println!("  🌿 {}", branch);
            }
            println!();
        }

        let orphaned_files = plan.orphaned_state_files();
        if !orphaned_files.is_empty() {
            println!("Orphaned State Files ({}):", orphaned_files.len());
            for file in orphaned_files {
                println!("  📝 {}", file.display());
            }
            println!();
        }

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

    pub fn show_cleanup_results(&self, results: &CleanupResults) {
        if results.is_dry_run {
            return; // Dry run results are shown by show_cleanup_preview
        }

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

        if results.has_errors() {
            println!("\n⚠️  Some items couldn't be cleaned:");
            for error in &results.errors {
                println!("  • {}", error);
            }
        }

        if results.total_items_processed() == 0 {
            println!("✨ Your Para environment was already clean!");
        }
    }

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }
}