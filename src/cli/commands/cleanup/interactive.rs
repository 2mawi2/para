use crate::utils::Result;
use dialoguer::Confirm;
use std::path::PathBuf;

/// Handles user interaction for cleanup operations
pub struct InteractiveHandler;

/// Represents the cleanup plan with items to be cleaned
#[derive(Debug)]
pub struct CleanupPlan {
    pub stale_branches: Vec<String>,
    pub orphaned_state_files: Vec<PathBuf>,
    pub old_archives: Vec<String>,
}

impl CleanupPlan {
    pub fn new() -> Self {
        Self {
            stale_branches: Vec::new(),
            orphaned_state_files: Vec::new(),
            old_archives: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stale_branches.is_empty()
            && self.orphaned_state_files.is_empty()
            && self.old_archives.is_empty()
    }
}

impl InteractiveHandler {
    /// Display a dry run report showing what would be cleaned
    pub fn show_dry_run_report(plan: &CleanupPlan, auto_cleanup_days: Option<u32>) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        if !plan.stale_branches.is_empty() {
            println!("Stale Branches ({}):", plan.stale_branches.len());
            for branch in &plan.stale_branches {
                println!("  🌿 {}", branch);
            }
            println!();
        }

        if !plan.orphaned_state_files.is_empty() {
            println!(
                "Orphaned State Files ({}):",
                plan.orphaned_state_files.len()
            );
            for file in &plan.orphaned_state_files {
                println!("  📝 {}", file.display());
            }
            println!();
        }

        if !plan.old_archives.is_empty() {
            let days = auto_cleanup_days.unwrap_or(30);
            println!("Old Archives (older than {} days):", days);
            for archive in &plan.old_archives {
                println!("  📦 {}", archive);
            }
            println!();
        }
    }

    /// Confirm cleanup with user and return whether to proceed
    pub fn confirm_cleanup(plan: &CleanupPlan, auto_cleanup_days: Option<u32>) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

        let mut total_items = 0;

        if !plan.stale_branches.is_empty() {
            println!("  🌿 {} stale branches", plan.stale_branches.len());
            total_items += plan.stale_branches.len();
        }

        if !plan.orphaned_state_files.is_empty() {
            println!(
                "  📝 {} orphaned state files",
                plan.orphaned_state_files.len()
            );
            total_items += plan.orphaned_state_files.len();
        }

        if !plan.old_archives.is_empty() {
            let days = auto_cleanup_days.unwrap_or(30);
            println!(
                "  📦 {} archived sessions (older than {} days)",
                plan.old_archives.len(),
                days
            );
            total_items += plan.old_archives.len();
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

    /// Check if running in non-interactive mode
    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }
}

/// Results of cleanup operations
#[derive(Debug, Default)]
pub struct CleanupResults {
    pub stale_branches_removed: usize,
    pub orphaned_state_files_removed: usize,
    pub old_archives_removed: usize,
    pub errors: Vec<String>,
}

impl CleanupResults {
    /// Display cleanup results to user
    pub fn show_results(&self) {
        println!("🧹 Cleanup Complete");
        println!("==================\n");

        if self.stale_branches_removed > 0 {
            println!(
                "  ✅ Removed {} stale branches",
                self.stale_branches_removed
            );
        }

        if self.orphaned_state_files_removed > 0 {
            println!(
                "  ✅ Removed {} orphaned state files",
                self.orphaned_state_files_removed
            );
        }

        if self.old_archives_removed > 0 {
            println!(
                "  ✅ Removed {} old archived sessions",
                self.old_archives_removed
            );
        }

        if !self.errors.is_empty() {
            println!("\n⚠️  Some items couldn't be cleaned:");
            for error in &self.errors {
                println!("  • {}", error);
            }
        }

        if self.stale_branches_removed == 0
            && self.orphaned_state_files_removed == 0
            && self.old_archives_removed == 0
        {
            println!("✨ Your Para environment was already clean!");
        }
    }
}
