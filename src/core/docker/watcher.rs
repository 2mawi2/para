//! Signal file watcher for container-host communication
//!
//! This module implements a background thread that monitors signal files
//! created by containers and processes them accordingly.

use crate::config::Config;
use crate::core::docker::signal_files::{
    delete_signal_file, read_signal_file, CancelSignal, ContainerStatus, FinishSignal,
    SignalFilePaths,
};
use crate::core::docker::DockerManager;
use crate::core::git::{FinishRequest, GitOperations, GitService};
#[cfg(test)]
use crate::core::session::SessionState;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Commands that can be sent to the watcher thread
#[derive(Debug)]
pub enum WatcherCommand {
    Stop,
}

/// Watcher state for signal file monitoring
pub struct SignalFileWatcher {
    session_name: String,
    worktree_path: PathBuf,
    config: Config,
    command_rx: Receiver<WatcherCommand>,
    stop_tx: Sender<()>,
}

/// Handle to control the watcher thread
pub struct WatcherHandle {
    command_tx: Sender<WatcherCommand>,
    thread_handle: Option<thread::JoinHandle<()>>,
    #[cfg(test)]
    stop_rx: Arc<Mutex<Receiver<()>>>,
}

impl WatcherHandle {
    /// Stop the watcher thread gracefully
    pub fn stop(mut self) -> Result<()> {
        // Send stop command
        let _ = self.command_tx.send(WatcherCommand::Stop);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            handle
                .join()
                .map_err(|_| ParaError::worktree_operation("Watcher thread panicked"))?;
        }

        Ok(())
    }

    /// Check if the watcher has stopped (used by tests)
    #[cfg(test)]
    pub fn has_stopped(&self) -> bool {
        if let Ok(stop_rx) = self.stop_rx.lock() {
            stop_rx.try_recv().is_ok()
        } else {
            false
        }
    }
}

impl SignalFileWatcher {
    /// Create and start a new signal file watcher
    pub fn spawn(session_name: String, worktree_path: PathBuf, config: Config) -> WatcherHandle {
        let (command_tx, command_rx) = mpsc::channel();
        let (stop_tx, _stop_rx) = mpsc::channel();
        #[cfg(test)]
        let stop_rx = Arc::new(Mutex::new(_stop_rx));
        #[cfg(test)]
        let stop_rx_clone = Arc::clone(&stop_rx);

        let watcher = SignalFileWatcher {
            session_name,
            worktree_path,
            config,
            command_rx,
            stop_tx,
        };

        let thread_handle = thread::spawn(move || {
            if let Err(e) = watcher.run() {
                eprintln!("Signal file watcher error: {e}");
            }
        });

        WatcherHandle {
            command_tx,
            thread_handle: Some(thread_handle),
            #[cfg(test)]
            stop_rx: stop_rx_clone,
        }
    }

    /// Main watcher loop
    fn run(self) -> Result<()> {
        let signal_paths = SignalFilePaths::new(&self.worktree_path);
        let poll_interval = Duration::from_secs(1);

        loop {
            // Check for commands
            if let Ok(cmd) = self.command_rx.try_recv() {
                match cmd {
                    WatcherCommand::Stop => {
                        let _ = self.stop_tx.send(());
                        return Ok(());
                    }
                }
            }

            // Check for finish signal
            if let Some(finish_signal) = read_signal_file::<FinishSignal>(&signal_paths.finish)? {
                self.handle_finish_signal(finish_signal)?;
                delete_signal_file(&signal_paths.finish)?;
                let _ = self.stop_tx.send(());
                return Ok(());
            }

            // Check for cancel signal
            if let Some(cancel_signal) = read_signal_file::<CancelSignal>(&signal_paths.cancel)? {
                self.handle_cancel_signal(cancel_signal)?;
                delete_signal_file(&signal_paths.cancel)?;
                let _ = self.stop_tx.send(());
                return Ok(());
            }

            // Check for status update
            if let Some(status) = read_signal_file::<ContainerStatus>(&signal_paths.status)? {
                self.handle_status_update(status)?;
                // Status files are not deleted, just overwritten
            }

            thread::sleep(poll_interval);
        }
    }

    /// Handle finish signal from container
    fn handle_finish_signal(&self, signal: FinishSignal) -> Result<()> {
        println!(
            "📦 Container finish signal received: {}",
            signal.commit_message
        );

        // Discover git repository from worktree
        let git_service = GitService::discover_from(&self.worktree_path)?;

        // Stage all changes
        println!("📦 Staging all changes in worktree...");
        git_service.stage_all_changes()?;

        // Create finish request
        let mut session_manager = SessionManager::new(&self.config);
        let session = session_manager.load_state(&self.session_name)?;

        let finish_request = FinishRequest {
            feature_branch: session.branch.clone(),
            commit_message: signal.commit_message.clone(),
            target_branch_name: signal.branch,
        };

        // Perform git finish
        let result = git_service.finish_session(finish_request)?;

        // Update session status
        session_manager.update_session_status(&self.session_name, SessionStatus::Review)?;

        // Stop the container
        let docker_manager = DockerManager::new(self.config.clone(), false, vec![]);
        if let Err(e) = docker_manager.stop_container(&self.session_name) {
            eprintln!("Warning: Failed to stop container: {e}");
        }

        match result {
            crate::core::git::FinishResult::Success { final_branch } => {
                println!("✓ Container session finished successfully");
                println!("  Feature branch: {final_branch}");
                println!("  Commit message: {}", signal.commit_message);
            }
        }

        Ok(())
    }

    /// Handle cancel signal from container
    fn handle_cancel_signal(&self, signal: CancelSignal) -> Result<()> {
        println!("📦 Container cancel signal received");

        let mut session_manager = SessionManager::new(&self.config);
        let session = session_manager.load_state(&self.session_name)?;

        // Cancel the session
        if signal.force {
            session_manager.cancel_session(&session.name, true)?;
        } else {
            // Check for uncommitted changes
            let git_service = GitService::discover_from(&self.worktree_path)?;
            if git_service.repository().has_uncommitted_changes()? {
                return Err(ParaError::git_operation(
                    "Container has uncommitted changes. Use --force to discard them.",
                ));
            }
            session_manager.cancel_session(&session.name, false)?;
        }

        // Stop the container
        let docker_manager = DockerManager::new(self.config.clone(), false, vec![]);
        if let Err(e) = docker_manager.stop_container(&self.session_name) {
            eprintln!("Warning: Failed to stop container: {e}");
        }

        println!("✓ Container session cancelled");

        Ok(())
    }

    /// Handle status update from container
    fn handle_status_update(&self, container_status: ContainerStatus) -> Result<()> {
        use crate::core::status::{Status, TestStatus};
        use chrono::Utc;

        // Parse container status fields into proper types
        let test_status = container_status
            .tests
            .as_ref()
            .and_then(|s| Status::parse_test_status(s).ok())
            .unwrap_or(TestStatus::Unknown);

        let (todos_completed, todos_total) = container_status
            .todos
            .as_ref()
            .and_then(|s| Status::parse_todos(s).ok())
            .map(|(completed, total)| (Some(completed), Some(total)))
            .unwrap_or((None, None));

        // Create status update
        let mut status = Status::new(
            self.session_name.clone(),
            container_status.task.clone(),
            test_status,
        );

        status.is_blocked = container_status.blocked;
        status.todos_completed = todos_completed;
        status.todos_total = todos_total;
        status.last_update = Utc::now();

        // Calculate diff stats from the worktree
        // This runs on the host so it can access the worktree files
        // Load the session state to get branch info
        let session_manager = SessionManager::new(&self.config);
        if let Ok(session_state) = session_manager.load_state(&self.session_name) {
            // Use the shared function to calculate diff stats
            if let Ok(diff_stats) = Status::calculate_diff_stats_for_session(&session_state) {
                status.diff_stats = Some(diff_stats);
            }
        }

        // Save to state directory in the main repository
        // This ensures the monitor can find it even when running from a worktree
        let state_dir = if self.config.directories.state_dir.starts_with('/') {
            // Absolute path
            std::path::PathBuf::from(&self.config.directories.state_dir)
        } else {
            // Relative path - resolve to main repository root
            match crate::utils::git::get_main_repository_root_from(Some(&self.worktree_path)) {
                Ok(repo_root) => repo_root.join(&self.config.directories.state_dir),
                Err(_) => {
                    // Fallback to relative path if we can't find repo root
                    std::path::PathBuf::from(&self.config.directories.state_dir)
                }
            }
        };

        status
            .save(&state_dir)
            .map_err(|e| ParaError::fs_error(format!("Failed to save container status: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().to_path_buf();
        let config = create_test_config();

        // Create .para directory
        fs::create_dir_all(worktree_path.join(".para")).unwrap();

        // Spawn watcher
        let handle =
            SignalFileWatcher::spawn("test-session".to_string(), worktree_path.clone(), config);

        // Give watcher time to start
        thread::sleep(Duration::from_millis(100));

        // Stop watcher
        handle.stop().unwrap();
    }

    #[test]
    fn test_finish_signal_detection() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let worktree_path = git_temp.path().join("test-worktree");

        // Create worktree
        git_service
            .create_worktree("test-branch", &worktree_path)
            .unwrap();

        // Create .para directory
        fs::create_dir_all(worktree_path.join(".para")).unwrap();

        // Save session state
        let session_manager = SessionManager::new(&config);
        let session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            worktree_path.clone(),
        );
        session_manager.save_state(&session).unwrap();

        // Spawn watcher
        let handle = SignalFileWatcher::spawn(
            "test-session".to_string(),
            worktree_path.clone(),
            config.clone(),
        );

        // Give the watcher thread time to start
        thread::sleep(Duration::from_millis(100));

        // Create a test file and finish signal
        fs::write(worktree_path.join("test.txt"), "test content").unwrap();

        let signal_paths = SignalFilePaths::new(&worktree_path);
        let finish_signal = FinishSignal {
            commit_message: "Test commit".to_string(),
            branch: None,
        };
        crate::core::docker::signal_files::write_signal_file(&signal_paths.finish, &finish_signal)
            .unwrap();

        // Give the filesystem time to sync the atomic write
        thread::sleep(Duration::from_millis(50));

        // Wait for watcher to process signal
        let start = std::time::Instant::now();
        while !handle.has_stopped() && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(100));
        }

        // Verify signal was processed
        assert!(!signal_paths.finish.exists());
    }

    #[test]
    fn test_cancel_signal_detection() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let worktree_path = git_temp.path().join("test-worktree");

        // Create worktree
        git_service
            .create_worktree("test-branch", &worktree_path)
            .unwrap();

        // Create .para directory
        fs::create_dir_all(worktree_path.join(".para")).unwrap();

        // Save session state
        let session_manager = SessionManager::new(&config);
        let session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            worktree_path.clone(),
        );
        session_manager.save_state(&session).unwrap();

        // Spawn watcher
        let handle = SignalFileWatcher::spawn(
            "test-session".to_string(),
            worktree_path.clone(),
            config.clone(),
        );

        // Give the watcher thread time to start
        thread::sleep(Duration::from_millis(100));

        // Create cancel signal
        let signal_paths = SignalFilePaths::new(&worktree_path);
        let cancel_signal = CancelSignal { force: true };
        crate::core::docker::signal_files::write_signal_file(&signal_paths.cancel, &cancel_signal)
            .unwrap();

        // Give the filesystem time to sync the atomic write
        thread::sleep(Duration::from_millis(50));

        // Wait for watcher to process signal
        let start = std::time::Instant::now();
        while !handle.has_stopped() && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(100));
        }

        // Verify signal was processed
        assert!(!signal_paths.cancel.exists());
    }
}
