use crate::cli::parser::CleanArgs;
use crate::cli::commands::clean::analyzers::{CleanupPlan, CleanupItem, CleanupItemType};
use crate::cli::commands::clean::interaction::CleanupInteraction;
use crate::core::git::GitService;
use crate::utils::Result;
use std::fs;

#[derive(Debug, Default)]
pub struct CleanupResults {
    pub stale_branches_removed: usize,
    pub orphaned_state_files_removed: usize,
    pub old_archives_removed: usize,
    pub errors: Vec<String>,
}

pub enum CleanupStrategy {
    Interactive,
    Force,
    DryRun,
}

impl CleanupStrategy {
    pub fn from_args(args: &CleanArgs) -> Self {
        if args.dry_run {
            CleanupStrategy::DryRun
        } else if args.force {
            CleanupStrategy::Force
        } else {
            CleanupStrategy::Interactive
        }
    }

    pub fn execute(
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