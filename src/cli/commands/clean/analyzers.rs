use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CleanupItem {
    pub item_type: CleanupItemType,
    pub identifier: String,
    pub path: Option<PathBuf>,
    pub safety_level: SafetyLevel,
}

#[derive(Debug, Clone)]
pub enum CleanupItemType {
    StaleBranch,
    OrphanedStateFile,
    OldArchive,
}

#[derive(Debug, Clone)]
pub enum SafetyLevel {
    Safe,     // Can be deleted without user confirmation
    Caution,  // Should ask user for confirmation
    Dangerous, // Require explicit --force flag
}

pub trait CleanupAnalyzer {
    fn analyze(&self) -> Result<Vec<CleanupItem>>;
    fn safety_level(&self) -> SafetyLevel;
    fn description(&self) -> &'static str;
}

pub struct StaleBranchAnalyzer {
    git_service: GitService,
    config: Config,
}

impl StaleBranchAnalyzer {
    pub fn new(git_service: GitService, config: Config) -> Self {
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

pub struct OrphanedStateAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OrphanedStateAnalyzer {
    pub fn new(git_service: GitService, config: Config) -> Self {
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

pub struct OldArchiveAnalyzer {
    git_service: GitService,
    config: Config,
}

impl OldArchiveAnalyzer {
    pub fn new(git_service: GitService, config: Config) -> Self {
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

pub struct CleanupAnalyzerRegistry {
    analyzers: Vec<Box<dyn CleanupAnalyzer>>,
}

impl CleanupAnalyzerRegistry {
    pub fn new(git_service: GitService, config: Config) -> Self {
        let analyzers: Vec<Box<dyn CleanupAnalyzer>> = vec![
            Box::new(StaleBranchAnalyzer::new(git_service.clone(), config.clone())),
            Box::new(OrphanedStateAnalyzer::new(git_service.clone(), config.clone())),
            Box::new(OldArchiveAnalyzer::new(git_service, config)),
        ];

        Self { analyzers }
    }

    pub fn create_cleanup_plan(&self) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::new();

        for analyzer in &self.analyzers {
            let items = analyzer.analyze()?;
            plan.add_items(items);
        }

        Ok(plan)
    }
}

#[derive(Debug)]
pub struct CleanupPlan {
    pub items: Vec<CleanupItem>,
}

impl CleanupPlan {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_items(&mut self, items: Vec<CleanupItem>) {
        self.items.extend(items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn count_by_type(&self, item_type: &CleanupItemType) -> usize {
        self.items.iter().filter(|item| 
            matches!((&item.item_type, item_type), 
                (CleanupItemType::StaleBranch, CleanupItemType::StaleBranch) |
                (CleanupItemType::OrphanedStateFile, CleanupItemType::OrphanedStateFile) |
                (CleanupItemType::OldArchive, CleanupItemType::OldArchive)
            )
        ).count()
    }

    pub fn has_dangerous_operations(&self) -> bool {
        self.items.iter().any(|item| matches!(item.safety_level, SafetyLevel::Dangerous))
    }
}