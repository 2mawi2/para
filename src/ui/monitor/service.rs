use crate::config::Config;
use crate::core::session::{SessionManager, SessionStatus as CoreSessionStatus};
use crate::core::status::Status;
use crate::ui::monitor::activity::detect_last_activity;
use crate::ui::monitor::cache::ActivityCache;
use crate::ui::monitor::{SessionInfo, SessionStatus};
use crate::utils::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct SessionService {
    config: Config,
    activity_cache: ActivityCache,
    task_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl SessionService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            activity_cache: ActivityCache::new(5),
            task_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_sessions(&self, show_stale: bool) -> Result<Vec<SessionInfo>> {
        let (sessions, current_session) = self.load_base_sessions()?;
        let sessions = self.enrich_with_activity(sessions)?;
        let sessions = self.enrich_with_tasks(sessions)?;
        let sessions = self.enrich_with_agent_status(sessions)?;
        let sessions = self.apply_filtering_and_sorting(sessions, show_stale, &current_session)?;
        Ok(sessions)
    }

    fn load_base_sessions(
        &self,
    ) -> Result<(
        Vec<crate::core::session::SessionState>,
        Option<crate::core::session::SessionState>,
    )> {
        let session_manager = SessionManager::new(&self.config);
        let sessions = session_manager.list_sessions()?;

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let current_session = session_manager
            .find_session_by_path(&current_dir)
            .unwrap_or(None);

        Ok((sessions, current_session))
    }

    fn enrich_with_activity(
        &self,
        sessions: Vec<crate::core::session::SessionState>,
    ) -> Result<Vec<(crate::core::session::SessionState, SessionInfo)>> {
        let mut enriched_sessions = Vec::new();

        for session in sessions {
            if matches!(session.status, CoreSessionStatus::Cancelled) {
                continue;
            }

            let last_activity = {
                let path = session.worktree_path.clone();

                if let Some(cached) = self.activity_cache.get(&path) {
                    cached
                } else {
                    let detected = detect_last_activity(&path);
                    self.activity_cache.set(path, detected);
                    detected
                }
            }
            .or(session.last_activity)
            .unwrap_or(session.created_at);

            let status = detect_session_status(&session, &last_activity);

            let session_info = SessionInfo {
                name: session.name.clone(),
                branch: session.branch.clone(),
                status,
                last_activity,
                task: format!("Session: {}", &session.name), // Will be properly set in enrich_with_tasks
                worktree_path: session.worktree_path.clone(),
                test_status: None,
                confidence: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            };

            enriched_sessions.push((session, session_info));
        }

        Ok(enriched_sessions)
    }

    fn enrich_with_tasks(
        &self,
        session_pairs: Vec<(crate::core::session::SessionState, SessionInfo)>,
    ) -> Result<Vec<SessionInfo>> {
        let mut session_infos = Vec::new();

        for (session, mut session_info) in session_pairs {
            let task = session.task_description.clone().unwrap_or_else(|| {
                // Check cache first
                let cache = self.task_cache.lock().unwrap();
                if let Some(cached_task) = cache.get(&session.name) {
                    cached_task.clone()
                } else {
                    drop(cache);
                    let state_dir = Path::new(&self.config.directories.state_dir);
                    let task_file = state_dir.join(format!("{}.task", session.name));
                    let task = std::fs::read_to_string(task_file)
                        .unwrap_or_else(|_| format!("Session: {}", &session.name));
                    let mut cache = self.task_cache.lock().unwrap();
                    cache.insert(session.name.clone(), task.clone());
                    task
                }
            });

            session_info.task = task;
            session_infos.push(session_info);
        }

        Ok(session_infos)
    }

    fn enrich_with_agent_status(&self, mut sessions: Vec<SessionInfo>) -> Result<Vec<SessionInfo>> {
        let state_dir = Path::new(&self.config.directories.state_dir);

        for session_info in &mut sessions {
            let agent_status = Status::load(state_dir, &session_info.name).ok().flatten();

            let (test_status, confidence, diff_stats, todo_percentage, is_blocked, agent_task) =
                if let Some(ref status) = agent_status {
                    (
                        Some(status.test_status.clone()),
                        Some(status.confidence.clone()),
                        status.diff_stats.clone(),
                        status.todo_percentage(),
                        status.is_blocked,
                        Some(status.current_task.clone()),
                    )
                } else {
                    (None, None, None, None, false, None)
                };

            // Agent task takes priority over session task
            if let Some(agent_task) = agent_task {
                session_info.task = agent_task;
            }

            session_info.test_status = test_status;
            session_info.confidence = confidence;
            session_info.diff_stats = diff_stats;
            session_info.todo_percentage = todo_percentage;
            session_info.is_blocked = is_blocked;
        }

        Ok(sessions)
    }

    fn apply_filtering_and_sorting(
        &self,
        mut sessions: Vec<SessionInfo>,
        show_stale: bool,
        current_session: &Option<crate::core::session::SessionState>,
    ) -> Result<Vec<SessionInfo>> {
        // Filter out stale sessions if requested
        if !show_stale {
            sessions.retain(|session_info| !matches!(session_info.status, SessionStatus::Stale));
        }

        // Sort by current session first, then by last activity
        sessions.sort_by(|a, b| {
            if let Some(ref current) = current_session {
                if a.name == current.name {
                    return std::cmp::Ordering::Less;
                }
                if b.name == current.name {
                    return std::cmp::Ordering::Greater;
                }
            }
            b.last_activity.cmp(&a.last_activity)
        });

        Ok(sessions)
    }
}

fn detect_session_status(
    session: &crate::core::session::SessionState,
    last_activity: &DateTime<Utc>,
) -> SessionStatus {
    // Check if session is marked as review
    if matches!(session.status, CoreSessionStatus::Review) {
        return SessionStatus::Review;
    }

    // Check if session is marked as finished (legacy)
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
    fn test_detect_session_status_review() {
        let mut session = SessionState::new(
            "review-session".to_string(),
            "review-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );
        session.update_status(CoreSessionStatus::Review);

        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Review));
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

    #[test]
    fn test_agent_status_integration() {
        use crate::core::status::{ConfidenceLevel, Status, TestStatus};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create an agent status file
        let agent_status = Status::new(
            "test-session".to_string(),
            "Agent is working on authentication".to_string(),
            TestStatus::Passed,
            ConfidenceLevel::High,
        )
        .with_todos(3, 7)
        .with_blocked(Some("Need help with Redis".to_string()));

        agent_status.save(&state_dir).unwrap();

        // Test status loading logic (mimicking what load_sessions does)
        let loaded_status = Status::load(&state_dir, "test-session").ok().flatten();
        assert!(loaded_status.is_some());

        let status = loaded_status.unwrap();

        // Test the tuple extraction logic
        let (test_status, confidence, todo_percentage, is_blocked, agent_task) = (
            Some(status.test_status.clone()),
            Some(status.confidence.clone()),
            status.todo_percentage(),
            status.is_blocked,
            Some(status.current_task.clone()),
        );

        assert_eq!(test_status, Some(TestStatus::Passed));
        assert_eq!(confidence, Some(ConfidenceLevel::High));
        assert_eq!(todo_percentage, Some(43)); // 3/7 = 43%
        assert!(is_blocked);
        assert_eq!(
            agent_task,
            Some("Agent is working on authentication".to_string())
        );

        // Test task priority logic (agent task over session task)
        let session_task = "Session default task".to_string();
        let final_task = agent_task.unwrap_or(session_task);
        assert_eq!(final_task, "Agent is working on authentication");
    }

    #[test]
    fn test_agent_status_fallback() {
        use crate::core::status::Status;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // No agent status file exists
        let loaded_status = Status::load(&state_dir, "nonexistent-session")
            .ok()
            .flatten();
        assert!(loaded_status.is_none());

        // Test fallback values when no agent status
        let (test_status, confidence, todo_percentage, is_blocked, agent_task) =
            if let Some(ref status) = loaded_status {
                (
                    Some(status.test_status.clone()),
                    Some(status.confidence.clone()),
                    status.todo_percentage(),
                    status.is_blocked,
                    Some(status.current_task.clone()),
                )
            } else {
                (None, None, None, false, None)
            };

        assert_eq!(test_status, None);
        assert_eq!(confidence, None);
        assert_eq!(todo_percentage, None);
        assert!(!is_blocked);
        assert_eq!(agent_task, None);

        // Test task fallback to session task
        let session_task = "Session default task".to_string();
        let final_task = agent_task.unwrap_or(session_task.clone());
        assert_eq!(final_task, session_task);
    }

    #[test]
    fn test_session_info_construction_with_agent_status() {
        use crate::core::status::{ConfidenceLevel, Status, TestStatus};
        use crate::ui::monitor::{SessionInfo, SessionStatus};
        use chrono::Utc;
        use std::path::PathBuf;

        // Create test agent status
        let agent_status = Status::new(
            "integration-session".to_string(),
            "Complex integration task".to_string(),
            TestStatus::Failed,
            ConfidenceLevel::Low,
        )
        .with_todos(2, 10);

        // Test SessionInfo construction with agent status (mimicking load_sessions logic)
        let session_name = "integration-session".to_string();
        let final_task = agent_status.current_task.clone();

        let session_info = SessionInfo {
            name: session_name.clone(),
            branch: "test/branch".to_string(),
            status: SessionStatus::Active,
            last_activity: Utc::now(),
            task: final_task,
            worktree_path: PathBuf::from("/test/path"),
            test_status: Some(agent_status.test_status.clone()),
            confidence: Some(agent_status.confidence.clone()),
            diff_stats: None,
            todo_percentage: agent_status.todo_percentage(),
            is_blocked: agent_status.is_blocked,
        };

        // Verify agent status is properly integrated
        assert_eq!(session_info.name, "integration-session");
        assert_eq!(session_info.task, "Complex integration task"); // Agent task priority
        assert_eq!(session_info.test_status, Some(TestStatus::Failed));
        assert_eq!(session_info.confidence, Some(ConfidenceLevel::Low));
        assert_eq!(session_info.todo_percentage, Some(20)); // 2/10 = 20%
        assert!(!session_info.is_blocked); // Agent status not blocked
    }

    #[test]
    fn test_current_session_keeps_actual_status() {
        // Test that current session is no longer forced to Active status
        use crate::core::session::SessionState;
        use chrono::Utc;

        // Create a session that would be Idle based on time
        let session = SessionState::new(
            "current-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );

        // Test that status is based on activity time, not current session
        let ten_minutes_ago = Utc::now() - chrono::Duration::minutes(10);
        let status = detect_session_status(&session, &ten_minutes_ago);

        // Should be Idle based on time, not forced to Active
        assert!(matches!(status, SessionStatus::Idle));
    }

    #[test]
    fn test_session_sorting_with_current_session() {
        use crate::ui::monitor::{SessionInfo, SessionStatus};
        use chrono::{Duration, Utc};
        use std::path::PathBuf;

        let now = Utc::now();

        // Create test sessions
        let session1 = SessionInfo {
            name: "session1".to_string(),
            branch: "branch1".to_string(),
            status: SessionStatus::Idle,
            last_activity: now - Duration::hours(1),
            task: "Task 1".to_string(),
            worktree_path: PathBuf::from("/test1"),
            test_status: None,
            confidence: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };

        let session2 = SessionInfo {
            name: "current-session".to_string(),
            branch: "branch2".to_string(),
            status: SessionStatus::Active,
            last_activity: now - Duration::hours(2), // Older than session1
            task: "Current Task".to_string(),
            worktree_path: PathBuf::from("/test2"),
            test_status: None,
            confidence: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };

        let session3 = SessionInfo {
            name: "session3".to_string(),
            branch: "branch3".to_string(),
            status: SessionStatus::Stale,
            last_activity: now, // Most recent
            task: "Task 3".to_string(),
            worktree_path: PathBuf::from("/test3"),
            test_status: None,
            confidence: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };

        let mut sessions = vec![session1, session2, session3];
        let current_session_name = Some("current-session".to_string());

        // Test sorting logic from load_sessions
        sessions.sort_by(|a, b| {
            if let Some(ref current) = current_session_name {
                if a.name == *current {
                    return std::cmp::Ordering::Less; // Current session goes first
                }
                if b.name == *current {
                    return std::cmp::Ordering::Greater; // Current session goes first
                }
            }
            b.last_activity.cmp(&a.last_activity) // Then by last activity
        });

        // Current session should be first despite being older
        assert_eq!(sessions[0].name, "current-session");
        // Then by most recent activity
        assert_eq!(sessions[1].name, "session3"); // Most recent
        assert_eq!(sessions[2].name, "session1"); // Least recent
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
            docker: crate::config::DockerConfig {
                enabled: false,
                mount_workspace: true,
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
            docker: crate::config::DockerConfig {
                enabled: false,
                mount_workspace: true,
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
