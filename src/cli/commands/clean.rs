use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

// Strategy pattern implementation for cleanup operations
trait CleanupStrategy {
    fn analyze(&self, config: &Config, git_service: &GitService) -> Result<CleanupPlan>;
    fn execute(&self, plan: CleanupPlan, git_service: &GitService) -> Result<CleanupResult>;
    fn cleanup_type(&self) -> &'static str;
}

#[derive(Debug, Default)]
struct CleanupPlan {
    items: Vec<CleanupItem>,
}

#[derive(Debug)]
enum CleanupItem {
    StaleBranch(String),
    OrphanedStateFile(PathBuf),
    OldArchive(String),
}

#[derive(Debug, Default)]
struct CleanupResult {
    items_removed: usize,
    errors: Vec<String>,
}

impl CleanupPlan {
    fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn add_stale_branch(&mut self, branch: String) {
        self.items.push(CleanupItem::StaleBranch(branch));
    }

    fn add_orphaned_state_file(&mut self, file: PathBuf) {
        self.items.push(CleanupItem::OrphanedStateFile(file));
    }

    fn add_old_archive(&mut self, archive: String) {
        self.items.push(CleanupItem::OldArchive(archive));
    }

    fn stale_branches(&self) -> impl Iterator<Item = &String> {
        self.items.iter().filter_map(|item| {
            if let CleanupItem::StaleBranch(branch) = item {
                Some(branch)
            } else {
                None
            }
        })
    }

    fn orphaned_state_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.items.iter().filter_map(|item| {
            if let CleanupItem::OrphanedStateFile(file) = item {
                Some(file)
            } else {
                None
            }
        })
    }

    fn old_archives(&self) -> impl Iterator<Item = &String> {
        self.items.iter().filter_map(|item| {
            if let CleanupItem::OldArchive(archive) = item {
                Some(archive)
            } else {
                None
            }
        })
    }

    fn count_by_type(&self) -> (usize, usize, usize) {
        let mut stale_branches = 0;
        let mut orphaned_files = 0;
        let mut old_archives = 0;

        for item in &self.items {
            match item {
                CleanupItem::StaleBranch(_) => stale_branches += 1,
                CleanupItem::OrphanedStateFile(_) => orphaned_files += 1,
                CleanupItem::OldArchive(_) => old_archives += 1,
            }
        }

        (stale_branches, orphaned_files, old_archives)
    }
}

// Stale branch cleanup strategy
struct StaleBranchCleaner;

impl CleanupStrategy for StaleBranchCleaner {
    fn analyze(&self, config: &Config, git_service: &GitService) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();
        let prefix = format!("{}/", config.git.branch_prefix);
        let state_dir = PathBuf::from(&config.directories.state_dir);

        let all_branches = git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    plan.add_stale_branch(branch_info.name);
                }
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: CleanupPlan, git_service: &GitService) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for branch in plan.stale_branches() {
            match git_service.delete_branch(branch, true) {
                Ok(_) => result.items_removed += 1,
                Err(e) => result
                    .errors
                    .push(format!("Failed to remove branch {}: {}", branch, e)),
            }
        }

        Ok(result)
    }

    fn cleanup_type(&self) -> &'static str {
        "stale branches"
    }
}

// Orphaned state file cleanup strategy
struct OrphanedStateCleaner;

impl OrphanedStateCleaner {
    fn scan_state_directory(&self, state_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut state_files = Vec::new();

        for entry in fs::read_dir(state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if self.is_state_file(&path) {
                state_files.push(path);
            }
        }

        Ok(state_files)
    }

    fn is_state_file(&self, path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| name.ends_with(".state"))
            .unwrap_or(false)
    }

    fn extract_session_id(&self, state_file: &std::path::Path) -> Result<String> {
        let file_name = state_file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| crate::utils::ParaError::invalid_args("Invalid state file name"))?;

        let session_id = file_name.strip_suffix(".state").ok_or_else(|| {
            crate::utils::ParaError::invalid_args("State file must end with .state")
        })?;

        Ok(session_id.to_string())
    }

    fn is_session_orphaned(&self, session_id: &str, config: &Config, git_service: &GitService) -> Result<bool> {
        let branch_name = format!("{}/{}", config.git.branch_prefix, session_id);
        Ok(!git_service.branch_exists(&branch_name)?)
    }

    fn find_related_files(&self, state_dir: &std::path::Path, session_id: &str) -> Vec<PathBuf> {
        let mut related_files = Vec::new();

        for suffix in &[".prompt", ".launch"] {
            let related_file = state_dir.join(format!("{}{}", session_id, suffix));
            if related_file.exists() {
                related_files.push(related_file);
            }
        }

        related_files
    }
}

impl CleanupStrategy for OrphanedStateCleaner {
    fn analyze(&self, config: &Config, git_service: &GitService) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();
        let state_dir = PathBuf::from(&config.directories.state_dir);

        if !state_dir.exists() {
            return Ok(plan);
        }

        let state_files = self.scan_state_directory(&state_dir)?;

        for state_file in state_files {
            let session_id = self.extract_session_id(&state_file)?;

            if self.is_session_orphaned(&session_id, config, git_service)? {
                plan.add_orphaned_state_file(state_file.clone());
                for related_file in self.find_related_files(&state_dir, &session_id) {
                    plan.add_orphaned_state_file(related_file);
                }
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: CleanupPlan, _git_service: &GitService) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for file_path in plan.orphaned_state_files() {
            match fs::remove_file(file_path) {
                Ok(_) => result.items_removed += 1,
                Err(e) => result.errors.push(format!(
                    "Failed to remove file {}: {}",
                    file_path.display(),
                    e
                )),
            }
        }

        Ok(result)
    }

    fn cleanup_type(&self) -> &'static str {
        "orphaned state files"
    }
}

// Archive cleanup strategy
struct ArchiveCleaner;

impl ArchiveCleaner {
    fn is_archive_older_than_cutoff(
        &self,
        branch: &str,
        cutoff_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        let timestamp_part = self.extract_archive_timestamp(branch)?;
        let branch_time = self.parse_archive_timestamp(&timestamp_part)?;
        Ok(branch_time.and_utc() < cutoff_date)
    }

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
        // Extract timestamp from branch name: prefix/archived/TIMESTAMP/name
        branch
            .split('/')
            .nth(2)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::utils::ParaError::invalid_args(format!(
                    "Invalid archived branch format: {}",
                    branch
                ))
            })
    }

    fn parse_archive_timestamp(&self, timestamp: &str) -> Result<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S").map_err(|e| {
            crate::utils::ParaError::invalid_args(format!(
                "Invalid timestamp format '{}': {}",
                timestamp, e
            ))
        })
    }
}

impl CleanupStrategy for ArchiveCleaner {
    fn analyze(&self, config: &Config, git_service: &GitService) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();
        
        let cleanup_days = match config.session.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(plan),
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = git_service
            .branch_manager()
            .list_archived_branches(&config.git.branch_prefix)?;

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                plan.add_old_archive(branch);
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: CleanupPlan, git_service: &GitService) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();

        for archive_branch in plan.old_archives() {
            match git_service.delete_branch(archive_branch, true) {
                Ok(_) => result.items_removed += 1,
                Err(e) => result.errors.push(format!(
                    "Failed to remove archive {}: {}",
                    archive_branch, e
                )),
            }
        }

        Ok(result)
    }

    fn cleanup_type(&self) -> &'static str {
        "old archives"
    }
}

// User interaction trait
trait UserInteraction {
    fn confirm_cleanup(&self, plan: &CleanupPlan, config: &Config) -> Result<bool>;
    fn show_dry_run_report(&self, plan: &CleanupPlan, config: &Config);
    fn show_results(&self, results: &CleanupResults);
}

struct InteractiveUI;
struct NonInteractiveUI;

impl UserInteraction for InteractiveUI {
    fn confirm_cleanup(&self, plan: &CleanupPlan, config: &Config) -> Result<bool> {
        println!("🧹 Para Cleanup");
        println!("===============\n");

        let (stale_branches, orphaned_files, old_archives) = plan.count_by_type();
        let mut total_items = 0;

        if stale_branches > 0 {
            println!("  🌿 {} stale branches", stale_branches);
            total_items += stale_branches;
        }

        if orphaned_files > 0 {
            println!("  📝 {} orphaned state files", orphaned_files);
            total_items += orphaned_files;
        }

        if old_archives > 0 {
            let days = config.session.auto_cleanup_days.unwrap_or(30);
            println!(
                "  📦 {} archived sessions (older than {} days)",
                old_archives, days
            );
            total_items += old_archives;
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

    fn show_dry_run_report(&self, plan: &CleanupPlan, config: &Config) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        let stale_branches: Vec<_> = plan.stale_branches().collect();
        let orphaned_files: Vec<_> = plan.orphaned_state_files().collect();
        let old_archives: Vec<_> = plan.old_archives().collect();

        if !stale_branches.is_empty() {
            println!("Stale Branches ({}):", stale_branches.len());
            for branch in stale_branches {
                println!("  🌿 {}", branch);
            }
            println!();
        }

        if !orphaned_files.is_empty() {
            println!("Orphaned State Files ({}):", orphaned_files.len());
            for file in orphaned_files {
                println!("  📝 {}", file.display());
            }
            println!();
        }

        if !old_archives.is_empty() {
            let days = config.session.auto_cleanup_days.unwrap_or(30);
            println!("Old Archives (older than {} days):", days);
            for archive in old_archives {
                println!("  📦 {}", archive);
            }
            println!();
        }
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

impl InteractiveUI {
    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }
}

impl UserInteraction for NonInteractiveUI {
    fn confirm_cleanup(&self, _plan: &CleanupPlan, _config: &Config) -> Result<bool> {
        // Non-interactive mode always proceeds without confirmation
        Ok(true)
    }

    fn show_dry_run_report(&self, plan: &CleanupPlan, config: &Config) {
        // Same as interactive for dry run
        let interactive = InteractiveUI;
        interactive.show_dry_run_report(plan, config);
    }

    fn show_results(&self, results: &CleanupResults) {
        // Same as interactive for results
        let interactive = InteractiveUI;
        interactive.show_results(results);
    }
}

// Cleanup orchestrator
struct CleanupOrchestrator {
    strategies: Vec<Box<dyn CleanupStrategy>>,
    user_interface: Box<dyn UserInteraction>,
}

impl CleanupOrchestrator {
    fn new(force: bool) -> Self {
        let strategies: Vec<Box<dyn CleanupStrategy>> = vec![
            Box::new(StaleBranchCleaner),
            Box::new(OrphanedStateCleaner),
            Box::new(ArchiveCleaner),
        ];

        let user_interface: Box<dyn UserInteraction> = if force || Self::is_non_interactive() {
            Box::new(NonInteractiveUI)
        } else {
            Box::new(InteractiveUI)
        };

        Self {
            strategies,
            user_interface,
        }
    }

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    fn analyze_cleanup(&self, config: &Config, git_service: &GitService) -> Result<CleanupPlan> {
        let mut combined_plan = CleanupPlan::new();

        for strategy in &self.strategies {
            let plan = strategy.analyze(config, git_service)?;
            // Merge plans
            for item in plan.items {
                combined_plan.items.push(item);
            }
        }

        Ok(combined_plan)
    }

    fn execute_cleanup(&self, config: &Config, git_service: &GitService, args: CleanArgs) -> Result<()> {
        let cleanup_plan = self.analyze_cleanup(config, git_service)?;

        if cleanup_plan.is_empty() {
            println!("🧹 Nothing to clean - your Para environment is already tidy!");
            return Ok(());
        }

        if args.dry_run {
            self.user_interface.show_dry_run_report(&cleanup_plan, config);
            return Ok(());
        }

        if !args.force && !self.user_interface.confirm_cleanup(&cleanup_plan, config)? {
            println!("Cleanup cancelled");
            return Ok(());
        }

        let results = self.perform_cleanup(cleanup_plan, git_service)?;
        self.user_interface.show_results(&results);

        Ok(())
    }

    fn perform_cleanup(&self, plan: CleanupPlan, git_service: &GitService) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        // Execute each strategy on its relevant items
        for strategy in &self.strategies {
            let strategy_plan = self.extract_strategy_plan(&plan, strategy.cleanup_type());
            let strategy_result = strategy.execute(strategy_plan, git_service)?;
            
            // Merge results based on strategy type
            match strategy.cleanup_type() {
                "stale branches" => results.stale_branches_removed += strategy_result.items_removed,
                "orphaned state files" => results.orphaned_state_files_removed += strategy_result.items_removed,
                "old archives" => results.old_archives_removed += strategy_result.items_removed,
                _ => {} // Unknown strategy type
            }
            
            results.errors.extend(strategy_result.errors);
        }

        Ok(results)
    }

    fn extract_strategy_plan(&self, plan: &CleanupPlan, strategy_type: &str) -> CleanupPlan {
        let mut strategy_plan = CleanupPlan::new();

        for item in &plan.items {
            let should_include = match (item, strategy_type) {
                (CleanupItem::StaleBranch(_), "stale branches") => true,
                (CleanupItem::OrphanedStateFile(_), "orphaned state files") => true,
                (CleanupItem::OldArchive(_), "old archives") => true,
                _ => false,
            };

            if should_include {
                strategy_plan.items.push(item.clone());
            }
        }

        strategy_plan
    }
}

// Add Clone to CleanupItem
impl Clone for CleanupItem {
    fn clone(&self) -> Self {
        match self {
            CleanupItem::StaleBranch(branch) => CleanupItem::StaleBranch(branch.clone()),
            CleanupItem::OrphanedStateFile(file) => CleanupItem::OrphanedStateFile(file.clone()),
            CleanupItem::OldArchive(archive) => CleanupItem::OldArchive(archive.clone()),
        }
    }
}

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;

    let orchestrator = CleanupOrchestrator::new(args.force);
    orchestrator.execute_cleanup(&config, &git_service, args)
}

// Legacy CleanupResults for backward compatibility with tests
#[derive(Debug, Default)]
struct CleanupResults {
    stale_branches_removed: usize,
    orphaned_state_files_removed: usize,
    old_archives_removed: usize,
    errors: Vec<String>,
}

// Legacy SessionCleaner for test compatibility
struct SessionCleaner {
    git_service: GitService,
    config: crate::config::Config,
}

impl SessionCleaner {
    fn new(git_service: GitService, config: crate::config::Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    fn find_stale_branches(&self) -> Result<Vec<String>> {
        let strategy = StaleBranchCleaner;
        let plan = strategy.analyze(&self.config, &self.git_service)?;
        Ok(plan.stale_branches().cloned().collect())
    }

    fn find_orphaned_state_files(&self) -> Result<Vec<PathBuf>> {
        let strategy = OrphanedStateCleaner;
        let plan = strategy.analyze(&self.config, &self.git_service)?;
        Ok(plan.orphaned_state_files().cloned().collect())
    }

    fn find_old_archives(&self) -> Result<Vec<String>> {
        let strategy = ArchiveCleaner;
        let plan = strategy.analyze(&self.config, &self.git_service)?;
        Ok(plan.old_archives().cloned().collect())
    }

    fn is_session_orphaned(&self, session_id: &str) -> Result<bool> {
        let strategy = OrphanedStateCleaner;
        strategy.is_session_orphaned(session_id, &self.config, &self.git_service)
    }

    fn extract_session_id(&self, state_file: &std::path::Path) -> Result<String> {
        let strategy = OrphanedStateCleaner;
        strategy.extract_session_id(state_file)
    }

    fn parse_archive_timestamp(&self, timestamp: &str) -> Result<chrono::NaiveDateTime> {
        let strategy = ArchiveCleaner;
        strategy.parse_archive_timestamp(timestamp)
    }

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
        let strategy = ArchiveCleaner;
        strategy.extract_archive_timestamp(branch)
    }

    fn is_state_file(&self, path: &std::path::Path) -> bool {
        let strategy = OrphanedStateCleaner;
        strategy.is_state_file(path)
    }

    fn find_related_files(&self, state_dir: &std::path::Path, session_id: &str) -> Vec<PathBuf> {
        let strategy = OrphanedStateCleaner;
        strategy.find_related_files(state_dir, session_id)
    }

    fn execute_clean(&self, args: CleanArgs) -> Result<()> {
        let orchestrator = CleanupOrchestrator::new(args.force);
        orchestrator.execute_cleanup(&self.config, &self.git_service, args)
    }

    fn perform_cleanup(&self, plan: LegacyCleanupPlan) -> Result<CleanupResults> {
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
}

// Legacy CleanupPlan for test compatibility 
#[derive(Debug)]
struct LegacyCleanupPlan {
    stale_branches: Vec<String>,
    orphaned_state_files: Vec<PathBuf>,
    old_archives: Vec<String>,
}

impl LegacyCleanupPlan {
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
    use crate::test_utils::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_plan_creation() {
        let plan = LegacyCleanupPlan::new();
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

    #[test]
    fn test_stale_branch_cleanup() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create a test branch without corresponding state file
        git_service.create_branch("test/stale-feature", "main").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let stale_branches = cleaner.find_stale_branches().unwrap();

        assert_eq!(stale_branches.len(), 1);
        assert_eq!(stale_branches[0], "test/stale-feature");
    }

    #[test]
    fn test_stale_branch_cleanup_with_existing_state() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create a test branch WITH corresponding state file
        git_service.create_branch("test/active-feature", "main").unwrap();
        fs::write(state_dir.join("active-feature.state"), "test state").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let stale_branches = cleaner.find_stale_branches().unwrap();

        // Should not be considered stale since state file exists
        assert!(stale_branches.is_empty());
    }

    #[test]
    fn test_orphaned_state_cleanup() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create orphaned state file (no corresponding branch)
        fs::write(state_dir.join("orphaned-session.state"), "test state").unwrap();
        fs::write(state_dir.join("orphaned-session.prompt"), "test prompt").unwrap();
        fs::write(state_dir.join("orphaned-session.launch"), "test launch").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let orphaned_files = cleaner.find_orphaned_state_files().unwrap();

        assert_eq!(orphaned_files.len(), 3); // .state, .prompt, .launch
        let file_names: std::collections::HashSet<String> = orphaned_files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        assert!(file_names.contains("orphaned-session.state"));
        assert!(file_names.contains("orphaned-session.prompt"));
        assert!(file_names.contains("orphaned-session.launch"));
    }

    #[test]
    fn test_orphaned_state_cleanup_with_existing_branch() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create branch and state file (not orphaned)
        git_service.create_branch("test/active-session", "main").unwrap();
        fs::write(state_dir.join("active-session.state"), "test state").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let orphaned_files = cleaner.find_orphaned_state_files().unwrap();

        // Should not be considered orphaned since branch exists
        assert!(orphaned_files.is_empty());
    }

    #[test]
    fn test_archive_cleanup() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();
        config.session.auto_cleanup_days = Some(7); // 7 days

        // Create an old archived branch (older than 7 days)
        let old_timestamp = "20240101-120000"; // Very old date
        let old_archive_branch = format!("test/archived/{}/old-feature", old_timestamp);
        git_service.create_branch(&old_archive_branch, "main").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let old_archives = cleaner.find_old_archives().unwrap();

        assert_eq!(old_archives.len(), 1);
        assert_eq!(old_archives[0], old_archive_branch);
    }

    #[test]
    fn test_archive_cleanup_no_auto_cleanup_days() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();
        config.session.auto_cleanup_days = None; // No auto-cleanup

        let cleaner = SessionCleaner::new(git_service, config);
        let old_archives = cleaner.find_old_archives().unwrap();

        // Should not find any archives when auto_cleanup_days is None
        assert!(old_archives.is_empty());
    }

    #[test]
    fn test_cleanup_with_confirmation_dry_run() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create stale branch for testing
        git_service.create_branch("test/stale-feature", "main").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let args = CleanArgs {
            force: false,
            dry_run: true,
            backups: false,
        };

        // Dry run should not fail and should not make changes
        let result = cleaner.execute_clean(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_non_interactive_mode_without_force() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create stale branch for testing
        git_service.create_branch("test/stale-feature", "main").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        
        // Set up non-interactive environment
        std::env::set_var("PARA_NON_INTERACTIVE", "1");
        
        let args = CleanArgs {
            force: false,
            dry_run: false,
            backups: false,
        };

        // Should fail in non-interactive mode without force flag
        let result = cleaner.execute_clean(args);
        assert!(result.is_err());
        
        // Clean up environment variable
        std::env::remove_var("PARA_NON_INTERACTIVE");
    }

    #[test]
    fn test_cleanup_non_interactive_mode_with_force() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create stale branch for testing
        git_service.create_branch("test/stale-feature", "main").unwrap();

        let cleaner = SessionCleaner::new(git_service.clone(), config);
        
        // Set up non-interactive environment
        std::env::set_var("PARA_NON_INTERACTIVE", "1");
        
        let args = CleanArgs {
            force: true,
            dry_run: false,
            backups: false,
        };

        // Should succeed in non-interactive mode with force flag
        let result = cleaner.execute_clean(args);
        assert!(result.is_ok());
        
        // Verify branch was actually deleted
        let branches = git_service.branch_manager().list_branches().unwrap();
        let stale_branch_exists = branches.iter().any(|b| b.name == "test/stale-feature");
        assert!(!stale_branch_exists);
        
        // Clean up environment variable
        std::env::remove_var("PARA_NON_INTERACTIVE");
    }

    #[test]
    fn test_cleanup_error_handling_with_partial_failures() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.git.branch_prefix = "test".to_string();

        // Create state directory and ensure it exists
        let state_dir = std::path::Path::new(&config.directories.state_dir);
        fs::create_dir_all(state_dir).unwrap();

        // Create a read-only state file to cause failure
        let readonly_file = state_dir.join("readonly-session.state");
        fs::write(&readonly_file, "test state").unwrap();
        let mut permissions = fs::metadata(&readonly_file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&readonly_file, permissions).unwrap();

        // Create a normal state file that should succeed
        fs::write(state_dir.join("normal-session.state"), "test state").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);
        let plan = LegacyCleanupPlan {
            stale_branches: Vec::new(),
            orphaned_state_files: vec![
                readonly_file.clone(),
                state_dir.join("normal-session.state"),
            ],
            old_archives: Vec::new(),
        };

        let results = cleaner.perform_cleanup(plan).unwrap();
        
        // Should have partial success
        assert_eq!(results.orphaned_state_files_removed, 1); // normal file removed
        assert_eq!(results.errors.len(), 1); // readonly file failed
        assert!(results.errors[0].contains("readonly-session.state"));
    }

    #[test]
    fn test_cleanup_empty_plan() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);
        
        let args = CleanArgs {
            force: false,
            dry_run: false,
            backups: false,
        };

        // Should succeed with empty plan
        let result = cleaner.execute_clean(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_session_id_from_state_file() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        // Test valid state file
        let state_file = temp_dir.path().join("test-session.state");
        let session_id = cleaner.extract_session_id(&state_file).unwrap();
        assert_eq!(session_id, "test-session");

        // Test invalid state file (no .state extension)
        let invalid_file = temp_dir.path().join("test-session.txt");
        let result = cleaner.extract_session_id(&invalid_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_session_orphaned() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.git.branch_prefix = "test".to_string();

        // Create a branch
        git_service.create_branch("test/existing-session", "main").unwrap();

        let cleaner = SessionCleaner::new(git_service, config);

        // Test existing session (not orphaned)
        let is_orphaned = cleaner.is_session_orphaned("existing-session").unwrap();
        assert!(!is_orphaned);

        // Test non-existing session (orphaned)
        let is_orphaned = cleaner.is_session_orphaned("non-existing-session").unwrap();
        assert!(is_orphaned);
    }

    #[test]
    fn test_parse_archive_timestamp() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        // Test valid timestamp
        let result = cleaner.parse_archive_timestamp("20250615-123456");
        assert!(result.is_ok());

        // Test invalid timestamp
        let result = cleaner.parse_archive_timestamp("invalid-timestamp");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_archive_timestamp() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        // Test valid archived branch
        let timestamp = cleaner.extract_archive_timestamp("test/archived/20250615-123456/feature-name");
        assert!(timestamp.is_ok());
        assert_eq!(timestamp.unwrap(), "20250615-123456");

        // Test invalid archived branch format
        let result = cleaner.extract_archive_timestamp("invalid/branch/format");
        assert!(result.is_err());
    }

    #[test] 
    fn test_is_state_file() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        // Test valid state file
        let state_file = temp_dir.path().join("test.state");
        assert!(cleaner.is_state_file(&state_file));

        // Test non-state file
        let text_file = temp_dir.path().join("test.txt");
        assert!(!cleaner.is_state_file(&text_file));

        // Test directory
        let dir = temp_dir.path().join("test-dir");
        fs::create_dir(&dir).unwrap();
        assert!(!cleaner.is_state_file(&dir));
    }

    #[test]
    fn test_find_related_files() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let cleaner = SessionCleaner::new(git_service, config);

        let state_dir = temp_dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        // Create related files
        fs::write(state_dir.join("test-session.prompt"), "prompt content").unwrap();
        fs::write(state_dir.join("test-session.launch"), "launch content").unwrap();
        
        let related_files = cleaner.find_related_files(&state_dir, "test-session");
        
        assert_eq!(related_files.len(), 2);
        let file_names: std::collections::HashSet<String> = related_files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        assert!(file_names.contains("test-session.prompt"));
        assert!(file_names.contains("test-session.launch"));
    }
}
