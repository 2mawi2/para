use crate::config::Config;
use crate::core::session::{SessionManager, SessionStatus as CoreSessionStatus};
use crate::ui::monitor::activity::detect_last_activity;
use crate::ui::monitor::cache::ActivityCache;
use crate::ui::monitor::{SessionInfo, SessionStatus};
use crate::utils::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Service for managing session data in the monitor UI
pub struct SessionService {
    config: Config,
    activity_cache: ActivityCache,
    task_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl SessionService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            // Cache activity detection results for 5 seconds
            activity_cache: ActivityCache::new(5),
            task_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_sessions(&self, show_stale: bool) -> Result<Vec<SessionInfo>> {
        let session_manager = SessionManager::new(&self.config);
        let sessions = session_manager.list_sessions()?;

        let mut session_infos = Vec::new();

        // Check if current directory is a session worktree
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let current_session = session_manager
            .find_session_by_path(&current_dir)
            .unwrap_or(None);

        for session in sessions {
            // Skip cancelled sessions
            if matches!(session.status, CoreSessionStatus::Cancelled) {
                continue;
            }

            // Get last activity from cache or detect it
            let last_activity = {
                let path = session.worktree_path.clone();

                // Check cache first
                if let Some(cached) = self.activity_cache.get(&path) {
                    cached
                } else {
                    // Not in cache, detect and store
                    let detected = detect_last_activity(&path);
                    self.activity_cache.set(path, detected);
                    detected
                }
            }
            .or(session.last_activity)
            .unwrap_or(session.created_at);

            // Detect status based on activity
            let mut status = detect_session_status(&session, &last_activity);

            // If this is the current session (where monitor was run from), mark it as active
            if let Some(ref current) = current_session {
                if current.name == session.name {
                    status = SessionStatus::Active;
                }
            }

            // Load task description with caching
            let task = session.task_description.clone().unwrap_or_else(|| {
                // Check cache first
                let cache = self.task_cache.lock().unwrap();
                if let Some(cached_task) = cache.get(&session.name) {
                    cached_task.clone()
                } else {
                    drop(cache); // Release lock before file I/O

                    // Try to load from task file for backward compatibility
                    let state_dir = Path::new(&self.config.directories.state_dir);
                    let task_file = state_dir.join(format!("{}.task", session.name));
                    let task = std::fs::read_to_string(task_file).unwrap_or_else(|_| {
                        // Show full session name if no task description
                        format!("Session: {}", &session.name)
                    });

                    // Cache the result
                    let mut cache = self.task_cache.lock().unwrap();
                    cache.insert(session.name.clone(), task.clone());
                    task
                }
            });

            let session_info = SessionInfo {
                name: session.name.clone(),
                branch: session.branch.clone(),
                status,
                last_activity,
                task,
                worktree_path: session.worktree_path.clone(),
            };

            // Filter out stale sessions if not showing them
            if !show_stale && matches!(session_info.status, SessionStatus::Stale) {
                continue;
            }

            session_infos.push(session_info);
        }

        // Sort by last activity (most recent first), but put current session first if found
        session_infos.sort_by(|a, b| {
            if let Some(ref current) = current_session {
                if a.name == current.name {
                    return std::cmp::Ordering::Less; // Current session goes first
                }
                if b.name == current.name {
                    return std::cmp::Ordering::Greater; // Current session goes first
                }
            }
            b.last_activity.cmp(&a.last_activity)
        });

        Ok(session_infos)
    }
}

fn detect_session_status(
    session: &crate::core::session::SessionState,
    last_activity: &DateTime<Utc>,
) -> SessionStatus {
    // Check if session is marked as finished
    if matches!(session.status, CoreSessionStatus::Finished) {
        return SessionStatus::Ready;
    }

    // Check activity time
    let now = Utc::now();
    let elapsed = now - *last_activity;

    match elapsed.num_minutes() {
        0..=5 => SessionStatus::Active,
        6..=1440 => SessionStatus::Idle, // 1440 minutes = 24 hours
        _ => SessionStatus::Stale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::session::SessionState;

    #[test]
    fn test_detect_session_status() {
        let session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );

        // Test active status (< 5 minutes)
        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Active));

        // Test idle status (10 minutes ago)
        let ten_minutes_ago = now - chrono::Duration::minutes(10);
        let status = detect_session_status(&session, &ten_minutes_ago);
        assert!(matches!(status, SessionStatus::Idle));

        // Test idle status (23 hours ago - still idle, not stale)
        let twenty_three_hours_ago = now - chrono::Duration::hours(23);
        let status = detect_session_status(&session, &twenty_three_hours_ago);
        assert!(matches!(status, SessionStatus::Idle));

        // Test stale status (> 24 hours)
        let twenty_five_hours_ago = now - chrono::Duration::hours(25);
        let status = detect_session_status(&session, &twenty_five_hours_ago);
        assert!(matches!(status, SessionStatus::Stale));
    }

    #[test]
    fn test_detect_session_status_ready() {
        let mut session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );
        session.update_status(CoreSessionStatus::Finished);

        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Ready));
    }

    #[test]
    fn test_service_activity_cache() {
        let config = create_test_config();
        let service = SessionService::new(config);

        // Mock path for testing
        let test_path = PathBuf::from("/test/worktree");

        // First call should detect activity (mock None for testing)
        let cached = service.activity_cache.get(&test_path);
        assert_eq!(cached, None, "Should not be cached initially");

        // Simulate caching
        service
            .activity_cache
            .set(test_path.clone(), Some(Utc::now()));

        // Second call should return cached value
        let cached = service.activity_cache.get(&test_path);
        assert!(cached.is_some(), "Should return cached value");
    }

    #[test]
    fn test_service_task_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create test task file
        let task_content = "Test task description";
        std::fs::write(state_dir.join("test-session.task"), task_content).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let service = SessionService::new(config);

        // Create a test session without task description
        let _session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            PathBuf::from("/test"),
        );

        // First access should read from file
        {
            let cache = service.task_cache.lock().unwrap();
            assert!(
                !cache.contains_key("test-session"),
                "Should not be cached initially"
            );
        }

        // Simulate the task loading logic
        let task = {
            let cache = service.task_cache.lock().unwrap();
            if let Some(cached) = cache.get("test-session") {
                cached.clone()
            } else {
                drop(cache);
                // Read from file
                let task_file = state_dir.join("test-session.task");
                let task = std::fs::read_to_string(task_file).unwrap();

                // Cache it
                let mut cache = service.task_cache.lock().unwrap();
                cache.insert("test-session".to_string(), task.clone());
                task
            }
        };

        assert_eq!(task, task_content);

        // Second access should use cache
        {
            let cache = service.task_cache.lock().unwrap();
            assert!(cache.contains_key("test-session"), "Should be cached now");
            assert_eq!(cache.get("test-session").unwrap(), task_content);
        }
    }

    #[test]
    fn test_task_cache_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let config = create_test_config();
        let service = Arc::new(SessionService::new(config));

        let mut handles = vec![];

        // Spawn multiple threads accessing the cache
        for i in 0..10 {
            let service_clone = Arc::clone(&service);
            let handle = thread::spawn(move || {
                let task_name = format!("task-{}", i);
                let task_content = format!("Task content {}", i);

                // Write to cache
                {
                    let mut cache = service_clone.task_cache.lock().unwrap();
                    cache.insert(task_name.clone(), task_content.clone());
                }

                // Read from cache
                {
                    let cache = service_clone.task_cache.lock().unwrap();
                    assert_eq!(cache.get(&task_name).unwrap(), &task_content);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all entries are in cache
        let cache = service.task_cache.lock().unwrap();
        for i in 0..10 {
            let task_name = format!("task-{}", i);
            assert!(cache.contains_key(&task_name));
        }
    }

    fn create_test_config() -> Config {
        Config {
            ide: crate::config::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "/tmp/subtrees".to_string(),
                state_dir: "/tmp/.para_state".to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_load_sessions() {
        let config = Config {
            ide: crate::config::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "/tmp/subtrees".to_string(),
                state_dir: "/tmp/.para_state_test".to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        };

        let service = SessionService::new(config);

        // Test loading sessions (should handle missing directory gracefully)
        let result = service.load_sessions(true);
        assert!(result.is_ok());

        // Test without stale sessions
        let result = service.load_sessions(false);
        assert!(result.is_ok());
    }
}
