use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Main Entry Point
// ============================================================================

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    
    let cleaner = SessionCleaner::new(git_service, config);
    cleaner.execute_clean(args)
}

// ============================================================================
// Main Session Cleaner - Simplified Logic
// ============================================================================

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

// ============================================================================
// Analyzer Framework - Trait and Implementations
// ============================================================================

#[derive(Debug, Clone)]
struct CleanupItem {
    item_type: CleanupItemType,
    identifier: String,
    path: Option<PathBuf>,
    safety_level: SafetyLevel,
}

#[derive(Debug, Clone)]
enum CleanupItemType {
    StaleBranch,
    OrphanedStateFile,
    OldArchive,
}

#[derive(Debug, Clone)]
enum SafetyLevel {
    Safe,     
    Caution,  
    Dangerous,
}

trait CleanupAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>>;
    fn safety_level(&self) -> SafetyLevel;
    fn description(&self) -> &'static str;
}

// Stale Branch Analyzer
struct StaleBranchAnalyzer {
    git_service: GitService,
    config: Config,
}

impl StaleBranchAnalyzer {
    fn new(git_service: GitService, config: Config) -> Self {
        Self { git_service, config }
    }
}

impl CleanupAnalyzer for StaleBranchAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let mut cleanup_items = Vec::new();
        let prefix = format!("{}/", self.config.git.branch_prefix);
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        let all_branches = self.git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    cleanup_items.push(CleanupItem {
                        item_type: CleanupItemType::StaleBranch,
                        identifier: branch_info.name,
                        path: None,
                        safety_level: SafetyLevel::Caution,
                    });
                }
            }
        }

        Ok(cleanup_items)
    }

    fn safety_level(&self) -> SafetyLevel {
        SafetyLevel::Caution
    }

    fn description(&self) -> &'static str {
        "Stale branches without corresponding state files"
    }
}

// Orphaned State Analyzer
struct OrphanedStateAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OrphanedStateAnalyzer {
    fn new(git_service: GitService, config: Config) -> Self {
        Self { git_service, config }
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

    fn is_session_orphaned(&self, session_id: &str) -> Result<bool> {
        let branch_name = format!("{}/{}", self.config.git.branch_prefix, session_id);
        Ok(!self.git_service.branch_exists(&branch_name)?)
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

impl CleanupAnalyzer for OrphanedStateAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        if !state_dir.exists() {
            return Ok(Vec::new());
        }

        let mut cleanup_items = Vec::new();

        for entry in fs::read_dir(&state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with(".state"))
                .unwrap_or(false)
            {
                let session_id = self.extract_session_id(&path)?;

                if self.is_session_orphaned(&session_id)? {
                    cleanup_items.push(CleanupItem {
                        item_type: CleanupItemType::OrphanedStateFile,
                        identifier: session_id.clone(),
                        path: Some(path.clone()),
                        safety_level: SafetyLevel::Safe,
                    });

                    // Add related files
                    for related_file in self.find_related_files(&state_dir, &session_id) {
                        cleanup_items.push(CleanupItem {
                            item_type: CleanupItemType::OrphanedStateFile,
                            identifier: format!("{} (related)", session_id),
                            path: Some(related_file),
                            safety_level: SafetyLevel::Safe,
                        });
                    }
                }
            }
        }

        Ok(cleanup_items)
    }

    fn safety_level(&self) -> SafetyLevel {
        SafetyLevel::Safe
    }

    fn description(&self) -> &'static str {
        "Orphaned state files without corresponding branches"
    }
}

// Old Archive Analyzer
struct OldArchiveAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OldArchiveAnalyzer {
    fn new(git_service: GitService, config: Config) -> Self {
        Self { git_service, config }
    }

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
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

    fn is_archive_older_than_cutoff(
        &self,
        branch: &str,
        cutoff_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        let timestamp_part = self.extract_archive_timestamp(branch)?;
        let branch_time = self.parse_archive_timestamp(&timestamp_part)?;
        Ok(branch_time.and_utc() < cutoff_date)
    }
}

impl CleanupAnalyzer for OldArchiveAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let cleanup_days = match self.config.session.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(Vec::new()),
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(&self.config.git.branch_prefix)?;

        let mut cleanup_items = Vec::new();

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                cleanup_items.push(CleanupItem {
                    item_type: CleanupItemType::OldArchive,
                    identifier: branch,
                    path: None,
                    safety_level: SafetyLevel::Caution,
                });
            }
        }

        Ok(cleanup_items)
    }

    fn safety_level(&self) -> SafetyLevel {
        SafetyLevel::Caution
    }

    fn description(&self) -> &'static str {
        "Old archived sessions beyond cleanup threshold"
    }
}

// Analyzer Registry
struct CleanupAnalyzerRegistry {
    analyzers: Vec<Box<dyn CleanupAnalyzer>>,
}

impl CleanupAnalyzerRegistry {
    fn new(git_service: GitService, config: Config) -> Self {
        let analyzers: Vec<Box<dyn CleanupAnalyzer>> = vec![
            Box::new(StaleBranchAnalyzer::new(git_service.clone(), config.clone())),
            Box::new(OrphanedStateAnalyzer::new(git_service.clone(), config.clone())),
            Box::new(OldArchiveAnalyzer::new(git_service, config)),
        ];

        Self { analyzers }
    }

    fn create_cleanup_plan(&self) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        for analyzer in &self.analyzers {
            let items = analyzer.analyze()?;
            plan.add_items(items);
        }

        Ok(plan)
    }
}

// ============================================================================
// Cleanup Plan and Strategy Framework
// ============================================================================

#[derive(Debug)]
struct CleanupPlan {
    items: Vec<CleanupItem>,
}

impl CleanupPlan {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn add_items(&mut self, items: Vec<CleanupItem>) {
        self.items.extend(items);
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn count_by_type(&self, item_type: &CleanupItemType) -> usize {
        self.items.iter().filter(|item| 
            matches!((&item.item_type, item_type), 
                (CleanupItemType::StaleBranch, CleanupItemType::StaleBranch) |
                (CleanupItemType::OrphanedStateFile, CleanupItemType::OrphanedStateFile) |
                (CleanupItemType::OldArchive, CleanupItemType::OldArchive)
            )
        ).count()
    }

    fn has_dangerous_operations(&self) -> bool {
        self.items.iter().any(|item| matches!(item.safety_level, SafetyLevel::Dangerous))
    }
}

#[derive(Debug, Default)]
struct CleanupResults {
    stale_branches_removed: usize,
    orphaned_state_files_removed: usize,
    old_archives_removed: usize,
    errors: Vec<String>,
}

enum CleanupStrategy {
    Interactive,
    Force,
    DryRun,
}

impl CleanupStrategy {
    fn from_args(args: &CleanArgs) -> Self {
        if args.dry_run {
            CleanupStrategy::DryRun
        } else if args.force {
            CleanupStrategy::Force
        } else {
            CleanupStrategy::Interactive
        }
    }

    fn execute(
        &self,
        plan: CleanupPlan,
        git_service: &GitService,
        interaction: &CleanupInteraction,
    ) -> Result<CleanupResults> {
        match self {
            CleanupStrategy::DryRun => {
                interaction.show_dry_run_report(&plan);
                Ok(CleanupResults::default())
            },
            CleanupStrategy::Force => {
                self.perform_cleanup(plan, git_service)
            },
            CleanupStrategy::Interactive => {
                if !interaction.confirm_cleanup(&plan)? {
                    println!("Cleanup cancelled");
                    return Ok(CleanupResults::default());
                }
                self.perform_cleanup(plan, git_service)
            },
        }
    }

    fn perform_cleanup(&self, plan: CleanupPlan, git_service: &GitService) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        for item in plan.items {
            match item.item_type {
                CleanupItemType::StaleBranch => {
                    match git_service.delete_branch(&item.identifier, true) {
                        Ok(_) => results.stale_branches_removed += 1,
                        Err(e) => results.errors.push(format!(
                            "Failed to remove branch {}: {}",
                            item.identifier, e
                        )),
                    }
                },
                CleanupItemType::OrphanedStateFile => {
                    if let Some(path) = item.path {
                        match fs::remove_file(&path) {
                            Ok(_) => results.orphaned_state_files_removed += 1,
                            Err(e) => results.errors.push(format!(
                                "Failed to remove file {}: {}",
                                path.display(), e
                            )),
                        }
                    }
                },
                CleanupItemType::OldArchive => {
                    match git_service.delete_branch(&item.identifier, true) {
                        Ok(_) => results.old_archives_removed += 1,
                        Err(e) => results.errors.push(format!(
                            "Failed to remove archive {}: {}",
                            item.identifier, e
                        )),
                    }
                },
            }
        }

        Ok(results)
    }
}

// ============================================================================
// User Interaction Framework
// ============================================================================

struct CleanupInteraction {
    config: Config,
}

impl CleanupInteraction {
    fn new(config: Config) -> Self {
        Self { config }
    }

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    fn confirm_cleanup(&self, plan: &CleanupPlan) -> Result<bool> {
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

    fn show_dry_run_report(&self, plan: &CleanupPlan) {
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

    fn show_cleanup_results(&self, results: &CleanupResults) {
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

// ============================================================================
// Tests
// ============================================================================

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

    #[test]
    fn test_cleanup_plan_creation() {
        let plan = CleanupPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.count_by_type(&CleanupItemType::StaleBranch), 0);
        assert_eq!(plan.count_by_type(&CleanupItemType::OrphanedStateFile), 0);
        assert_eq!(plan.count_by_type(&CleanupItemType::OldArchive), 0);
    }

    #[test]
    fn test_cleanup_results_default() {
        let results = CleanupResults::default();
        assert_eq!(results.stale_branches_removed, 0);
        assert_eq!(results.orphaned_state_files_removed, 0);
        assert_eq!(results.old_archives_removed, 0);
        assert!(results.errors.is_empty());
    }
}