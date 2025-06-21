use crate::cli::commands::clean::analyzers::{CleanupPlan, CleanupItemType};
use crate::cli::commands::clean::strategies::CleanupResults;
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

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    pub fn confirm_cleanup(&self, plan: &CleanupPlan) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

        let mut total_items = 0;

        let stale_branches_count = plan.count_by_type(&CleanupItemType::StaleBranch);
        let orphaned_files_count = plan.count_by_type(&CleanupItemType::OrphanedStateFile);
        let old_archives_count = plan.count_by_type(&CleanupItemType::OldArchive);

        if stale_branches_count > 0 {
            println!("  🌿 {} stale branches", stale_branches_count);
            total_items += stale_branches_count;
        }

        if orphaned_files_count > 0 {
            println!("  📝 {} orphaned state files", orphaned_files_count);
            total_items += orphaned_files_count;
        }

        if old_archives_count > 0 {
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!(
                "  📦 {} archived sessions (older than {} days)",
                old_archives_count, days
            );
            total_items += old_archives_count;
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

    pub fn show_dry_run_report(&self, plan: &CleanupPlan) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        let stale_branches: Vec<_> = plan.items.iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::StaleBranch))
            .collect();

        let orphaned_files: Vec<_> = plan.items.iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OrphanedStateFile))
            .collect();

        let old_archives: Vec<_> = plan.items.iter()
            .filter(|item| matches!(item.item_type, CleanupItemType::OldArchive))
            .collect();

        if !stale_branches.is_empty() {
            println!("Stale Branches ({}):", stale_branches.len());
            for item in stale_branches {
                println!("  🌿 {}", item.identifier);
            }
            println!();
        }

        if !orphaned_files.is_empty() {
            println!("Orphaned State Files ({}):", orphaned_files.len());
            for item in orphaned_files {
                if let Some(path) = &item.path {
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
            for item in old_archives {
                println!("  📦 {}", item.identifier);
            }
            println!();
        }
    }

    pub fn show_cleanup_results(&self, results: &CleanupResults) {
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