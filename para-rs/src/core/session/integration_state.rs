use crate::cli::parser::IntegrationStrategy;
use crate::utils::error::{ParaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationState {
    pub session_id: String,
    pub feature_branch: String,
    pub base_branch: String,
    pub strategy: IntegrationStrategy,
    pub conflict_files: Vec<PathBuf>,
    pub commit_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub step: IntegrationStep,
    pub original_head_commit: Option<String>,
    pub original_working_dir: Option<PathBuf>,
    pub backup_branch: Option<String>,
    pub temp_branches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationStep {
    Started,
    BaseBranchUpdated,
    FeatureBranchPrepared,
    ConflictsDetected { files: Vec<PathBuf> },
    ConflictsResolved,
    IntegrationComplete,
    Failed { error: String },
}

impl IntegrationState {
    pub fn new(
        session_id: String,
        feature_branch: String,
        base_branch: String,
        strategy: IntegrationStrategy,
        commit_message: Option<String>,
    ) -> Self {
        Self {
            session_id,
            feature_branch,
            base_branch,
            strategy,
            conflict_files: Vec::new(),
            commit_message,
            created_at: Utc::now(),
            step: IntegrationStep::Started,
            original_head_commit: None,
            original_working_dir: None,
            backup_branch: None,
            temp_branches: Vec::new(),
        }
    }

    pub fn with_backup_info(
        mut self,
        original_head: String,
        working_dir: PathBuf,
        backup_branch: String,
    ) -> Self {
        self.original_head_commit = Some(original_head);
        self.original_working_dir = Some(working_dir);
        self.backup_branch = Some(backup_branch);
        self
    }

    pub fn add_temp_branch(&mut self, branch_name: String) {
        self.temp_branches.push(branch_name);
    }

    pub fn with_conflicts(mut self, files: Vec<PathBuf>) -> Self {
        self.conflict_files = files.clone();
        self.step = IntegrationStep::ConflictsDetected { files };
        self
    }

    pub fn mark_step(&mut self, step: IntegrationStep) {
        self.step = step;
    }

    pub fn is_in_conflict(&self) -> bool {
        matches!(self.step, IntegrationStep::ConflictsDetected { .. })
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.step, IntegrationStep::IntegrationComplete)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.step, IntegrationStep::Failed { .. })
    }
}

pub struct IntegrationStateManager {
    state_dir: PathBuf,
}

impl IntegrationStateManager {
    pub fn new(state_dir: PathBuf) -> Self {
        Self { state_dir }
    }

    pub fn ensure_state_dir(&self) -> Result<()> {
        if !self.state_dir.exists() {
            fs::create_dir_all(&self.state_dir).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to create state directory {}: {}",
                    self.state_dir.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    pub fn save_integration_state(&self, state: &IntegrationState) -> Result<()> {
        self.ensure_state_dir()?;

        let state_file = self.get_integration_state_file();
        let state_json = serde_json::to_string_pretty(state).map_err(|e| {
            ParaError::serialization(format!("Failed to serialize integration state: {}", e))
        })?;

        fs::write(&state_file, state_json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to write integration state to {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(())
    }

    pub fn load_integration_state(&self) -> Result<Option<IntegrationState>> {
        let state_file = self.get_integration_state_file();

        if !state_file.exists() {
            return Ok(None);
        }

        let state_json = fs::read_to_string(&state_file).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to read integration state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        let state: IntegrationState = serde_json::from_str(&state_json).map_err(|e| {
            ParaError::serialization(format!("Failed to deserialize integration state: {}", e))
        })?;

        Ok(Some(state))
    }

    pub fn clear_integration_state(&self) -> Result<()> {
        let state_file = self.get_integration_state_file();

        if state_file.exists() {
            fs::remove_file(&state_file).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to remove integration state file {}: {}",
                    state_file.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    pub fn has_active_integration(&self) -> bool {
        self.get_integration_state_file().exists()
    }

    pub fn update_integration_step(&self, step: IntegrationStep) -> Result<()> {
        if let Some(mut state) = self.load_integration_state()? {
            state.mark_step(step);
            self.save_integration_state(&state)?;
        }
        Ok(())
    }

    fn get_integration_state_file(&self) -> PathBuf {
        self.state_dir.join("integration_state.json")
    }

    pub fn cleanup_all_state(&self) -> Result<()> {
        let integration_file = self.get_integration_state_file();
        let conflict_dir = self.state_dir.join("conflicts");
        let backup_dir = self.state_dir.join("backups");

        for file in [&integration_file] {
            if file.exists() {
                fs::remove_file(file).map_err(|e| {
                    ParaError::file_operation(format!(
                        "Failed to remove state file {}: {}",
                        file.display(),
                        e
                    ))
                })?;
            }
        }

        for dir in [&conflict_dir, &backup_dir] {
            if dir.exists() {
                fs::remove_dir_all(dir).map_err(|e| {
                    ParaError::file_operation(format!(
                        "Failed to remove state directory {}: {}",
                        dir.display(),
                        e
                    ))
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_state_manager() -> (TempDir, IntegrationStateManager) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let state_dir = temp_dir.path().join(".para_state");
        let manager = IntegrationStateManager::new(state_dir);
        (temp_dir, manager)
    }

    #[test]
    fn test_save_and_load_integration_state() {
        let (_temp_dir, manager) = setup_state_manager();

        let state = IntegrationState::new(
            "test-session".to_string(),
            "feature-branch".to_string(),
            "main".to_string(),
            IntegrationStrategy::Merge,
            Some("Test commit message".to_string()),
        );

        manager
            .save_integration_state(&state)
            .expect("Failed to save state");
        assert!(manager.has_active_integration());

        let loaded_state = manager
            .load_integration_state()
            .expect("Failed to load state");
        assert!(loaded_state.is_some());

        let loaded = loaded_state.unwrap();
        assert_eq!(loaded.session_id, "test-session");
        assert_eq!(loaded.feature_branch, "feature-branch");
        assert_eq!(loaded.base_branch, "main");
        assert!(matches!(loaded.strategy, IntegrationStrategy::Merge));
        assert_eq!(
            loaded.commit_message,
            Some("Test commit message".to_string())
        );
    }

    #[test]
    fn test_integration_state_with_conflicts() {
        let (_temp_dir, manager) = setup_state_manager();

        let conflicts = vec![PathBuf::from("src/file1.rs"), PathBuf::from("src/file2.rs")];

        let state = IntegrationState::new(
            "conflict-session".to_string(),
            "feature".to_string(),
            "main".to_string(),
            IntegrationStrategy::Squash,
            None,
        )
        .with_conflicts(conflicts.clone());

        assert!(state.is_in_conflict());
        assert!(!state.is_complete());
        assert_eq!(state.conflict_files, conflicts);

        manager
            .save_integration_state(&state)
            .expect("Failed to save state");

        let loaded = manager
            .load_integration_state()
            .expect("Failed to load state")
            .unwrap();
        assert!(loaded.is_in_conflict());
        assert_eq!(loaded.conflict_files, conflicts);
    }

    #[test]
    fn test_update_integration_step() {
        let (_temp_dir, manager) = setup_state_manager();

        let state = IntegrationState::new(
            "step-test".to_string(),
            "feature".to_string(),
            "main".to_string(),
            IntegrationStrategy::Rebase,
            None,
        );

        manager
            .save_integration_state(&state)
            .expect("Failed to save state");

        manager
            .update_integration_step(IntegrationStep::BaseBranchUpdated)
            .expect("Failed to update step");

        let loaded = manager
            .load_integration_state()
            .expect("Failed to load state")
            .unwrap();
        assert!(matches!(loaded.step, IntegrationStep::BaseBranchUpdated));
    }

    #[test]
    fn test_clear_integration_state() {
        let (_temp_dir, manager) = setup_state_manager();

        let state = IntegrationState::new(
            "clear-test".to_string(),
            "feature".to_string(),
            "main".to_string(),
            IntegrationStrategy::Merge,
            None,
        );

        manager
            .save_integration_state(&state)
            .expect("Failed to save state");
        assert!(manager.has_active_integration());

        manager
            .clear_integration_state()
            .expect("Failed to clear state");
        assert!(!manager.has_active_integration());

        let loaded = manager
            .load_integration_state()
            .expect("Failed to load state");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_cleanup_all_state() {
        let (_temp_dir, manager) = setup_state_manager();

        let state = IntegrationState::new(
            "cleanup-test".to_string(),
            "feature".to_string(),
            "main".to_string(),
            IntegrationStrategy::Merge,
            None,
        );

        manager
            .save_integration_state(&state)
            .expect("Failed to save state");
        assert!(manager.has_active_integration());

        manager
            .cleanup_all_state()
            .expect("Failed to cleanup state");
        assert!(!manager.has_active_integration());

        let loaded = manager
            .load_integration_state()
            .expect("Failed to load state");
        assert!(loaded.is_none());
    }
}
