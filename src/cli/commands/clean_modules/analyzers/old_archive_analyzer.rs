use super::{CleanupAnalyzer, CleanupItem};
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub struct OldArchiveAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OldArchiveAnalyzer {
    pub fn new(git_service: GitService, config: Config) -> Self {
        Self {
            git_service,
            config,
        }
    }
}

impl CleanupAnalyzer for OldArchiveAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>> {
        let items = Vec::new();
        // TODO: Implement old archive detection
        Ok(items)
    }

    fn description(&self) -> &'static str {
        "Finds old archived sessions that can be removed"
    }
}