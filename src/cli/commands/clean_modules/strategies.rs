use super::analyzers::{CleanupItem, CleanupPlan};
use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use std::fs;

#[derive(Debug, Clone)]
pub enum CleanupStrategy {
    Interactive,
    Force,
    DryRun,
}

impl CleanupStrategy {
    pub fn from_args(args: &CleanArgs) -> Self {
        if args.dry_run {
            Self::DryRun
        } else if args.force {
            Self::Force
        } else {
            Self::Interactive
        }
    }

    pub fn execute(
        &self,
        plan: CleanupPlan,
        git_service: &GitService,
        config: &Config,
    ) -> Result<CleanupResults> {
        match self {
            Self::DryRun => Ok(CleanupResults::dry_run()),
            Self::Interactive | Self::Force => self.perform_cleanup(plan, git_service, config),
        }
    }

    fn perform_cleanup(
        &self,
        plan: CleanupPlan,
        git_service: &GitService,
        _config: &Config,
    ) -> Result<CleanupResults> {
        let mut results = CleanupResults::default();

        for item in plan.items() {
            match item {
                CleanupItem::StaleBranch { name } => {
                    match git_service.delete_branch(name, true) {
                        Ok(_) => results.stale_branches_removed += 1,
                        Err(e) => results
                            .errors
                            .push(format!("Failed to remove branch {}: {}", name, e)),
                    }
                }
                CleanupItem::OrphanedStateFile { path } => {
                    match fs::remove_file(path) {
                        Ok(_) => results.orphaned_state_files_removed += 1,
                        Err(e) => results.errors.push(format!(
                            "Failed to remove file {}: {}",
                            path.display(),
                            e
                        )),
                    }
                }
                CleanupItem::OldArchive { name } => {
                    match git_service.delete_branch(name, true) {
                        Ok(_) => results.old_archives_removed += 1,
                        Err(e) => results.errors.push(format!(
                            "Failed to remove archive {}: {}",
                            name, e
                        )),
                    }
                }
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Default)]
pub struct CleanupResults {
    pub stale_branches_removed: usize,
    pub orphaned_state_files_removed: usize,
    pub old_archives_removed: usize,
    pub errors: Vec<String>,
    pub is_dry_run: bool,
}

impl CleanupResults {
    pub fn dry_run() -> Self {
        Self {
            is_dry_run: true,
            ..Default::default()
        }
    }

    pub fn total_items_processed(&self) -> usize {
        self.stale_branches_removed + self.orphaned_state_files_removed + self.old_archives_removed
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_successful(&self) -> bool {
        !self.has_errors() && self.total_items_processed() > 0
    }
}