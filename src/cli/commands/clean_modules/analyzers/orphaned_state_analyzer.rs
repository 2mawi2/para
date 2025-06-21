use super::{CleanupAnalyzer, CleanupItem};
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub struct OrphanedStateAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OrphanedStateAnalyzer {
    pub fn new(git_service: GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }
}

impl CleanupAnalyzer for OrphanedStateAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let items = Vec::new();
        // TODO: Implement orphaned state file detection
        Ok(items)
    }

    fn description(&self) -> &'static str {
        "Finds state files without corresponding branches"
    }
}