mod old_archive_analyzer;
mod orphaned_state_analyzer;
mod stale_branch_analyzer;

pub use old_archive_analyzer::OldArchiveAnalyzer;
pub use orphaned_state_analyzer::OrphanedStateAnalyzer;
pub use stale_branch_analyzer::StaleBranchAnalyzer;

use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use std::path::PathBuf;

pub trait CleanupAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>>;
    fn description(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum CleanupItem {
    StaleBranch { name: String },
    OrphanedStateFile { path: PathBuf },
    OldArchive { name: String },
}

#[derive(Debug, Default)]
pub struct CleanupPlan {
    items: Vec<CleanupItem>,
}

impl CleanupPlan {
    pub fn new(items: Vec<CleanupItem>) -> Self {
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[CleanupItem] {
        &self.items
    }

    pub fn stale_branches(&self) -> Vec<&String> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::StaleBranch { name } => Some(name),
                _ => None,
            })
            .collect()
    }

    pub fn orphaned_state_files(&self) -> Vec<&PathBuf> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::OrphanedStateFile { path } => Some(path),
                _ => None,
            })
            .collect()
    }

    pub fn old_archives(&self) -> Vec<&String> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CleanupItem::OldArchive { name } => Some(name),
                _ => None,
            })
            .collect()
    }
}

pub struct CleanupAnalyzerRegistry {
    analyzers: Vec<Box<dyn CleanupAnalyzer>>,
}

impl CleanupAnalyzerRegistry {
    pub fn new(git_service: &GitService, config: &Config) -> Self {
        let mut analyzers: Vec<Box<dyn CleanupAnalyzer>> = Vec::new();
        
        analyzers.push(Box::new(StaleBranchAnalyzer::new(git_service.clone())));
        analyzers.push(Box::new(OrphanedStateAnalyzer::new(
            git_service.clone(),
            config.clone(),
        )));
        analyzers.push(Box::new(OldArchiveAnalyzer::new(
            git_service.clone(),
            config.clone(),
        )));

        Self { analyzers }
    }

    pub fn create_cleanup_plan(&self) -> Result<CleanupPlan> {
        let mut all_items = Vec::new();

        for analyzer in &self.analyzers {
            let items = analyzer.analyze()?;
            all_items.extend(items);
        }

        Ok(CleanupPlan::new(all_items))
    }
}