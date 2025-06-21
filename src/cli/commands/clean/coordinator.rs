use super::strategies::{CleanupItem, CleanupStrategy};
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct CleanupResults {
    pub stale_branches_removed: usize,
    pub orphaned_state_files_removed: usize,
    pub old_archives_removed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct CleanupPlan {
    pub items: Vec<CleanupItem>,
}

impl Default for CleanupPlan {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanupPlan {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn stale_branches(&self) -> Vec<&String> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::StaleBranch(branch) => Some(branch),
                _ => None,
            })
            .collect()
    }

    pub fn orphaned_state_files(&self) -> Vec<&PathBuf> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::OrphanedStateFile(path) => Some(path),
                _ => None,
            })
            .collect()
    }

    pub fn old_archives(&self) -> Vec<&String> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::OldArchive(archive) => Some(archive),
                _ => None,
            })
            .collect()
    }
}

pub struct CleanupCoordinator {
    strategies: Vec<Box<dyn CleanupStrategy>>,
}

impl CleanupCoordinator {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn add_strategy(mut self, strategy: Box<dyn CleanupStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub fn analyze_cleanup(
        &self,
        config: &Config,
        git_service: &GitService,
    ) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        for strategy in &self.strategies {
            let mut items = strategy.find_cleanup_items(config, git_service)?;
            plan.items.append(&mut items);
        }

        Ok(plan)
    }

    pub fn perform_cleanup(
        &self,
        plan: CleanupPlan,
        git_service: &GitService,
    ) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        for item in plan.items {
            match self.clean_single_item(&item, git_service) {
                Ok(_) => self.increment_success_counter(&mut results, &item),
                Err(e) => results.errors.push(self.format_error(&item, &e)),
            }
        }

        Ok(results)
    }

    #[cfg(test)]
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    fn clean_single_item(&self, item: &CleanupItem, git_service: &GitService) -> Result<()> {
        for strategy in &self.strategies {
            if strategy.clean_item(item, git_service).is_ok() {
                return Ok(());
            }
        }

        Err(crate::utils::ParaError::invalid_args(
            "No strategy available to clean this item",
        ))
    }

    fn increment_success_counter(&self, results: &mut CleanupResults, item: &CleanupItem) {
        match item {
            CleanupItem::StaleBranch(_) => results.stale_branches_removed += 1,
            CleanupItem::OrphanedStateFile(_) => results.orphaned_state_files_removed += 1,
            CleanupItem::OldArchive(_) => results.old_archives_removed += 1,
        }
    }

    fn format_error(&self, item: &CleanupItem, error: &crate::utils::ParaError) -> String {
        match item {
            CleanupItem::StaleBranch(branch) => {
                format!("Failed to remove branch {}: {}", branch, error)
            }
            CleanupItem::OrphanedStateFile(path) => {
                format!("Failed to remove file {}: {}", path.display(), error)
            }
            CleanupItem::OldArchive(archive) => {
                format!("Failed to remove archive {}: {}", archive, error)
            }
        }
    }
}

impl Default for CleanupCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::clean::strategies::{
        ArchiveCleanupStrategy, StateFileCleanupStrategy, WorktreeCleanupStrategy,
    };
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_plan_creation() {
        let plan = CleanupPlan::new();
        assert!(plan.is_empty());
        assert!(plan.items.is_empty());
    }

    #[test]
    fn test_cleanup_plan_filtering() {
        let mut plan = CleanupPlan::new();
        plan.items
            .push(CleanupItem::StaleBranch("branch1".to_string()));
        plan.items
            .push(CleanupItem::OrphanedStateFile(PathBuf::from("file1.state")));
        plan.items
            .push(CleanupItem::OldArchive("archive1".to_string()));

        assert_eq!(plan.stale_branches().len(), 1);
        assert_eq!(plan.orphaned_state_files().len(), 1);
        assert_eq!(plan.old_archives().len(), 1);
        assert!(!plan.is_empty());
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
    fn test_coordinator_creation() {
        let coordinator = CleanupCoordinator::new();
        assert_eq!(coordinator.strategy_count(), 0);
    }

    #[test]
    fn test_coordinator_with_strategies() {
        let coordinator = CleanupCoordinator::new()
            .add_strategy(Box::new(WorktreeCleanupStrategy))
            .add_strategy(Box::new(StateFileCleanupStrategy))
            .add_strategy(Box::new(ArchiveCleanupStrategy));

        assert_eq!(coordinator.strategy_count(), 3);
    }

    #[test]
    fn test_coordinator_analyze_cleanup() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config();
        let coordinator = CleanupCoordinator::new().add_strategy(Box::new(WorktreeCleanupStrategy));

        let result = coordinator.analyze_cleanup(&config, &git_service);
        assert!(result.is_ok());
    }
}
