use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;

    let orchestrator = CleanupOrchestrator::new(git_service, config);
    orchestrator.execute_clean(args)
}

// Trait-based architecture for cleanup operations
trait ResourceCleaner {
    type Resource;

    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn find_resources(&self) -> Result<Vec<Self::Resource>>;
    fn clean_resource(&self, resource: &Self::Resource) -> Result<()>;
}

#[derive(Debug, Clone)]
enum CleanupResource {
    StaleBranch(String),
    OrphanedFile(PathBuf),
    OldArchive(String),
}

impl CleanupResource {
    #[allow(dead_code)]
    fn resource_type(&self) -> &'static str {
        match self {
            CleanupResource::StaleBranch(_) => "stale-branch",
            CleanupResource::OrphanedFile(_) => "orphaned-file",
            CleanupResource::OldArchive(_) => "old-archive",
        }
    }
}

struct StaleBranchCleaner<'a> {
    git_service: &'a GitService,
    config: &'a Config,
}

impl<'a> StaleBranchCleaner<'a> {
    fn new(git_service: &'a GitService, config: &'a Config) -> Self {
        Self {
            git_service,
            config,
        }
    }
}

impl<'a> ResourceCleaner for StaleBranchCleaner<'a> {
    type Resource = CleanupResource;

    fn name(&self) -> &str {
        "stale-branches"
    }

    fn find_resources(&self) -> Result<Vec<Self::Resource>> {
        let mut resources = Vec::new();
        let prefix = format!("{}/", self.config.git.branch_prefix);
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        let all_branches = self.git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{session_id}.state"));

                if !state_file.exists() {
                    resources.push(CleanupResource::StaleBranch(branch_info.name));
                }
            }
        }

        Ok(resources)
    }

    fn clean_resource(&self, resource: &Self::Resource) -> Result<()> {
        match resource {
            CleanupResource::StaleBranch(branch_name) => {
                self.git_service.delete_branch(branch_name, true)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "StaleBranchCleaner can only clean stale branches",
            )),
        }
    }
}

struct OrphanedFileCleaner<'a> {
    git_service: &'a GitService,
    config: &'a Config,
}

impl<'a> OrphanedFileCleaner<'a> {
    fn new(git_service: &'a GitService, config: &'a Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

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

    fn is_session_orphaned(&self, session_id: &str) -> Result<bool> {
        let branch_name = format!("{}/{}", self.config.git.branch_prefix, session_id);
        Ok(!self.git_service.branch_exists(&branch_name)?)
    }

    fn find_related_files(&self, state_dir: &std::path::Path, session_id: &str) -> Vec<PathBuf> {
        let mut related_files = Vec::new();

        for suffix in &[".prompt", ".launch"] {
            let related_file = state_dir.join(format!("{session_id}{suffix}"));
            if related_file.exists() {
                related_files.push(related_file);
            }
        }

        related_files
    }
}

impl<'a> ResourceCleaner for OrphanedFileCleaner<'a> {
    type Resource = CleanupResource;

    fn name(&self) -> &str {
        "orphaned-files"
    }

    fn find_resources(&self) -> Result<Vec<Self::Resource>> {
        let state_dir = PathBuf::from(&self.config.directories.state_dir);

        if !state_dir.exists() {
            return Ok(Vec::new());
        }

        let mut resources = Vec::new();
        let state_files = self.scan_state_directory(&state_dir)?;

        for state_file in state_files {
            let session_id = self.extract_session_id(&state_file)?;

            if self.is_session_orphaned(&session_id)? {
                resources.push(CleanupResource::OrphanedFile(state_file.clone()));
                for related_file in self.find_related_files(&state_dir, &session_id) {
                    resources.push(CleanupResource::OrphanedFile(related_file));
                }
            }
        }

        Ok(resources)
    }

    fn clean_resource(&self, resource: &Self::Resource) -> Result<()> {
        match resource {
            CleanupResource::OrphanedFile(file_path) => {
                fs::remove_file(file_path)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "OrphanedFileCleaner can only clean orphaned files",
            )),
        }
    }
}

struct ArchiveCleaner<'a> {
    git_service: &'a GitService,
    config: &'a Config,
}

impl<'a> ArchiveCleaner<'a> {
    fn new(git_service: &'a GitService, config: &'a Config) -> Self {
        Self {
            git_service,
            config,
        }
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

    fn extract_archive_timestamp(&self, branch: &str) -> Result<String> {
        branch
            .split('/')
            .nth(2)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::utils::ParaError::invalid_args(format!(
                    "Invalid archived branch format: {branch}"
                ))
            })
    }

    fn parse_archive_timestamp(&self, timestamp: &str) -> Result<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S").map_err(|e| {
            crate::utils::ParaError::invalid_args(format!(
                "Invalid timestamp format '{timestamp}': {e}"
            ))
        })
    }
}

impl<'a> ResourceCleaner for ArchiveCleaner<'a> {
    type Resource = CleanupResource;

    fn name(&self) -> &str {
        "old-archives"
    }

    fn find_resources(&self) -> Result<Vec<Self::Resource>> {
        let cleanup_days = match self.config.session.auto_cleanup_days {
            Some(days) => days,
            None => return Ok(Vec::new()),
        };

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(cleanup_days as i64);
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(&self.config.git.branch_prefix)?;

        let mut resources = Vec::new();

        for branch in archived_branches {
            if self.is_archive_older_than_cutoff(&branch, cutoff_date)? {
                resources.push(CleanupResource::OldArchive(branch));
            }
        }

        Ok(resources)
    }

    fn clean_resource(&self, resource: &Self::Resource) -> Result<()> {
        match resource {
            CleanupResource::OldArchive(branch_name) => {
                self.git_service.delete_branch(branch_name, true)?;
                Ok(())
            }
            _ => Err(crate::utils::ParaError::invalid_args(
                "ArchiveCleaner can only clean old archives",
            )),
        }
    }
}

struct CleanupOrchestrator {
    git_service: GitService,
    config: Config,
}

impl CleanupOrchestrator {
    fn new(git_service: GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }

    fn analyze_cleanup(&self) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        // Use specialized cleaners to find resources
        let stale_cleaner = StaleBranchCleaner::new(&self.git_service, &self.config);
        let orphaned_cleaner = OrphanedFileCleaner::new(&self.git_service, &self.config);
        let archive_cleaner = ArchiveCleaner::new(&self.git_service, &self.config);

        // Find stale branches
        let stale_resources = stale_cleaner.find_resources()?;
        for resource in stale_resources {
            if let CleanupResource::StaleBranch(branch_name) = resource {
                plan.stale_branches.push(branch_name);
            }
        }

        // Find orphaned files
        let orphaned_resources = orphaned_cleaner.find_resources()?;
        for resource in orphaned_resources {
            if let CleanupResource::OrphanedFile(file_path) = resource {
                plan.orphaned_state_files.push(file_path);
            }
        }

        // Find old archives
        let archive_resources = archive_cleaner.find_resources()?;
        for resource in archive_resources {
            if let CleanupResource::OldArchive(branch_name) = resource {
                plan.old_archives.push(branch_name);
            }
        }

        Ok(plan)
    }

    fn execute_cleanup(&self, plan: CleanupPlan) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        let stale_cleaner = StaleBranchCleaner::new(&self.git_service, &self.config);
        let orphaned_cleaner = OrphanedFileCleaner::new(&self.git_service, &self.config);
        let archive_cleaner = ArchiveCleaner::new(&self.git_service, &self.config);

        // Clean stale branches
        for branch in plan.stale_branches {
            let resource = CleanupResource::StaleBranch(branch.clone());
            match stale_cleaner.clean_resource(&resource) {
                Ok(_) => results.stale_branches_removed += 1,
                Err(e) => results
                    .errors
                    .push(format!("Failed to remove branch {branch}: {e}")),
            }
        }

        // Clean orphaned files
        for file_path in plan.orphaned_state_files {
            let resource = CleanupResource::OrphanedFile(file_path.clone());
            match orphaned_cleaner.clean_resource(&resource) {
                Ok(_) => results.orphaned_state_files_removed += 1,
                Err(e) => results.errors.push(format!(
                    "Failed to remove file {}: {e}",
                    file_path.display()
                )),
            }
        }

        // Clean old archives
        for archive_branch in plan.old_archives {
            let resource = CleanupResource::OldArchive(archive_branch.clone());
            match archive_cleaner.clean_resource(&resource) {
                Ok(_) => results.old_archives_removed += 1,
                Err(e) => results
                    .errors
                    .push(format!("Failed to remove archive {archive_branch}: {e}")),
            }
        }

        Ok(results)
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

        let results = self.execute_cleanup(cleanup_plan)?;
        self.show_results(&results);

        Ok(())
    }

    fn is_non_interactive() -> bool {
        std::env::var("PARA_NON_INTERACTIVE").is_ok()
            || std::env::var("CI").is_ok()
            || !atty::is(atty::Stream::Stdin)
    }

    fn show_dry_run_report(&self, plan: &CleanupPlan) {
        println!("🧹 Para Cleanup - Dry Run");
        println!("========================\n");

        if !plan.stale_branches.is_empty() {
            println!("Stale Branches ({}):", plan.stale_branches.len());
            for branch in &plan.stale_branches {
                println!("  🌿 {branch}");
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
            println!("Old Archives (older than {days} days):");
            for archive in &plan.old_archives {
                println!("  📦 {archive}");
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
                println!("  • {error}");
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

#[derive(Debug, Default)]
struct CleanupResults {
    stale_branches_removed: usize,
    orphaned_state_files_removed: usize,
    old_archives_removed: usize,
    errors: Vec<String>,
}

impl CleanupResults {
    #[allow(dead_code)]
    fn total_removed(&self) -> usize {
        self.stale_branches_removed + self.orphaned_state_files_removed + self.old_archives_removed
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
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

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

    // Tests for new trait-based architecture
    #[test]
    fn test_resource_cleaner_trait_implementation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let cleaner = StaleBranchCleaner::new(&git_service, &config);

        // Test that cleaner can find resources
        let resources = cleaner.find_resources().unwrap();
        assert!(resources.is_empty()); // No stale branches initially

        // Test cleaner name
        assert_eq!(cleaner.name(), "stale-branches");
    }

    #[test]
    fn test_orphaned_file_cleaner_implementation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let cleaner = OrphanedFileCleaner::new(&git_service, &config);

        // Test that cleaner can find resources
        let resources = cleaner.find_resources().unwrap();
        assert!(resources.is_empty()); // No orphaned files initially

        // Test cleaner name
        assert_eq!(cleaner.name(), "orphaned-files");
    }

    #[test]
    fn test_archive_cleaner_implementation() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.session.auto_cleanup_days = Some(30);

        let cleaner = ArchiveCleaner::new(&git_service, &config);

        // Test that cleaner can find resources
        let resources = cleaner.find_resources().unwrap();
        assert!(resources.is_empty()); // No old archives initially

        // Test cleaner name
        assert_eq!(cleaner.name(), "old-archives");
    }

    #[test]
    fn test_cleanup_orchestrator_coordination() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let orchestrator = CleanupOrchestrator::new(git_service, config);

        // Test that orchestrator can analyze cleanup needs
        let plan = orchestrator.analyze_cleanup().unwrap();
        assert!(plan.is_empty()); // No cleanup needed initially

        // Test that orchestrator can execute cleanup
        let results = orchestrator.execute_cleanup(plan).unwrap();
        assert_eq!(results.total_removed(), 0); // Nothing to clean initially
    }

    #[test]
    fn test_cleanup_orchestrator_with_stale_branches() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create state directory to ensure the path exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        // Create a stale branch (branch without state file)
        let stale_branch = format!("{}/stale-session", config.git.branch_prefix);
        let current_branch = git_service.repository().get_current_branch().unwrap();
        git_service
            .create_branch(&stale_branch, &current_branch)
            .unwrap();

        // Switch back to main branch so we can delete the stale branch
        git_service
            .repository()
            .checkout_branch(&current_branch)
            .unwrap();

        let orchestrator = CleanupOrchestrator::new(git_service, config);

        // Test that orchestrator finds the stale branch
        let plan = orchestrator.analyze_cleanup().unwrap();
        assert!(!plan.is_empty());

        // Test that orchestrator can clean it up
        let results = orchestrator.execute_cleanup(plan).unwrap();
        assert!(results.total_removed() > 0);
    }
}
