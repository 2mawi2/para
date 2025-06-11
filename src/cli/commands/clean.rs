use crate::cli::parser::CleanArgs;
use crate::config::ConfigManager;
use crate::core::git::{GitOperations, GitService};
use crate::utils::{ParaError, Result};
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

pub fn execute(args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load configuration: {}", e)))?;

    let cleaner = SessionCleaner::new(git_service, config);
    cleaner.execute_clean(args)
}

struct SessionCleaner {
    git_service: GitService,
    config: crate::config::Config,
}

#[derive(Debug, Default)]
struct CleanupResults {
    stale_branches_removed: usize,
    orphaned_state_files_removed: usize,
    old_archives_removed: usize,
    errors: Vec<String>,
}

impl SessionCleaner {
    fn new(git_service: GitService, config: crate::config::Config) -> Self {
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
            self.show_dry_run_report(&cleanup_plan);
            return Ok(());
        }

        if !args.force && !self.confirm_cleanup(&cleanup_plan)? {
            println!("Cleanup cancelled");
            return Ok(());
        }

        let results = self.perform_cleanup(cleanup_plan)?;
        self.show_results(&results);

        Ok(())
    }

    fn analyze_cleanup(&self) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        // Find stale branches (branches without corresponding state files)
        plan.stale_branches = self.find_stale_branches()?;

        // Find orphaned state files (state files without corresponding branches)
        plan.orphaned_state_files = self.find_orphaned_state_files()?;

        // Find old archives to clean up
        plan.old_archives = self.find_old_archives()?;

        Ok(plan)
    }

    fn find_stale_branches(&self) -> Result<Vec<String>> {
        let mut stale_branches = Vec::new();
        let prefix = format!("{}/", self.config.git.branch_prefix);
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        // Get all branches with our prefix
        let all_branches = self.git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    stale_branches.push(branch_info.name);
                }
            }
        }

        Ok(stale_branches)
    }

    fn find_orphaned_state_files(&self) -> Result<Vec<PathBuf>> {
        let mut orphaned_files = Vec::new();
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        if !state_dir.exists() {
            return Ok(orphaned_files);
        }

        for entry in fs::read_dir(&state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".state") {
                    if let Some(session_id) = file_name.strip_suffix(".state") {
                        let branch_name =
                            format!("{}/{}", self.config.git.branch_prefix, session_id);

                        if !self.git_service.branch_exists(&branch_name)? {
                            orphaned_files.push(path.clone());

                            // Also include related files (.prompt, .launch)
                            for suffix in &[".prompt", ".launch"] {
                                let related_file =
                                    state_dir.join(format!("{}{}", session_id, suffix));
                                if related_file.exists() {
                                    orphaned_files.push(related_file);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(orphaned_files)
    }

    fn find_old_archives(&self) -> Result<Vec<String>> {
        let mut old_archives = Vec::new();

        if let Some(cleanup_days) = self.config.session.auto_cleanup_days {
            let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
            let archived_branches = self
                .git_service
                .branch_manager()
                .list_archived_branches(&self.config.git.branch_prefix)?;

            for branch in archived_branches {
                // Extract timestamp from branch name: prefix/archived/TIMESTAMP/name
                if let Some(timestamp_part) = branch.split('/').nth(2) {
                    if let Ok(branch_time) =
                        chrono::NaiveDateTime::parse_from_str(timestamp_part, "%Y%m%d-%H%M%S")
                    {
                        if branch_time.and_utc() < cutoff_date {
                            old_archives.push(branch);
                        }
                    }
                }
            }
        }

        Ok(old_archives)
    }

    fn show_dry_run_report(&self, plan: &CleanupPlan) {
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
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
            println!("Old Archives (older than {} days):", days);
            for archive in &plan.old_archives {
                println!("  📦 {}", archive);
            }
            println!();
        }
    }

    fn confirm_cleanup(&self, plan: &CleanupPlan) -> Result<bool> {
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
            let days = self.config.session.auto_cleanup_days.unwrap_or(30);
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

        Ok(Confirm::new()
            .with_prompt("Continue with cleanup?")
            .default(false)
            .interact()
            .unwrap_or(false))
    }

    fn perform_cleanup(&self, plan: CleanupPlan) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        // Clean stale branches
        for branch in plan.stale_branches {
            match self.git_service.delete_branch(&branch, true) {
                Ok(_) => results.stale_branches_removed += 1,
                Err(e) => results
                    .errors
                    .push(format!("Failed to remove branch {}: {}", branch, e)),
            }
        }

        // Clean orphaned state files
        for file_path in plan.orphaned_state_files {
            match fs::remove_file(&file_path) {
                Ok(_) => results.orphaned_state_files_removed += 1,
                Err(e) => results.errors.push(format!(
                    "Failed to remove file {}: {}",
                    file_path.display(),
                    e
                )),
            }
        }

        // Clean old archives
        for archive_branch in plan.old_archives {
            match self.git_service.delete_branch(&archive_branch, true) {
                Ok(_) => results.old_archives_removed += 1,
                Err(e) => results.errors.push(format!(
                    "Failed to remove archive {}: {}",
                    archive_branch, e
                )),
            }
        }

        Ok(results)
    }

    fn show_results(&self, results: &CleanupResults) {
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

#[derive(Debug)]
struct CleanupPlan {
    stale_branches: Vec<String>,
    orphaned_state_files: Vec<PathBuf>,
    old_archives: Vec<String>,
}

impl CleanupPlan {
    fn new() -> Self {
        Self {
            stale_branches: Vec::new(),
            orphaned_state_files: Vec::new(),
            old_archives: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.stale_branches.is_empty()
            && self.orphaned_state_files.is_empty()
            && self.old_archives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_plan_creation() {
        let plan = CleanupPlan::new();
        assert!(plan.is_empty());
        assert!(plan.stale_branches.is_empty());
        assert!(plan.orphaned_state_files.is_empty());
        assert!(plan.old_archives.is_empty());
    }

    #[test]
    fn test_cleanup_results_default() {
        let results = CleanupResults::default();
        assert_eq!(results.stale_branches_removed, 0);
        assert_eq!(results.orphaned_state_files_removed, 0);
        assert_eq!(results.old_archives_removed, 0);
        assert!(results.errors.is_empty());
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
