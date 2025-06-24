//! Tests for mixed worktree and container sessions

#[cfg(test)]
mod tests {
    use crate::core::session::{SessionManager, SessionType};
    use crate::test_utils::test_helpers::*;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_session_state_worktree_type() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Create a worktree session state manually
        let worktree_path = temp_dir.path().join(".para/worktrees/test-worktree");
        let session = crate::core::session::SessionState::new(
            "test-worktree".to_string(),
            "para/test-worktree".to_string(),
            worktree_path,
        );

        // Verify it's a worktree type
        assert_eq!(session.session_type, SessionType::Worktree);
        assert!(!session.is_container());
        assert!(matches!(session.session_type, SessionType::Worktree));

        // Save and reload
        manager.save_state(&session).unwrap();
        let loaded = manager.load_state(&session.name).unwrap();
        assert_eq!(loaded.session_type, SessionType::Worktree);
    }

    #[test]
    fn test_session_state_container_type() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Create a container session state manually
        let worktree_path = temp_dir.path().join(".para/worktrees/test-container");
        let session = crate::core::session::SessionState::new_container_with_parent_branch(
            "test-container".to_string(),
            "para/test-container".to_string(),
            worktree_path,
            Some("abc123".to_string()),
            "main".to_string(),
        );

        // Verify it's a container type
        assert!(matches!(
            session.session_type,
            SessionType::Container { .. }
        ));
        assert!(session.is_container());
        // Verify container has the expected ID
        if let SessionType::Container { container_id } = &session.session_type {
            assert_eq!(container_id.as_deref(), Some("abc123"));
        } else {
            panic!("Expected container type");
        }

        // Save and reload
        manager.save_state(&session).unwrap();
        let loaded = manager.load_state(&session.name).unwrap();
        assert!(matches!(loaded.session_type, SessionType::Container { .. }));
        // Verify container has the expected ID
        if let SessionType::Container { container_id } = &loaded.session_type {
            assert_eq!(container_id.as_deref(), Some("abc123"));
        } else {
            panic!("Expected container type");
        }
    }

    #[test]
    fn test_list_mixed_sessions() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Create mixed session states
        let sessions = vec![
            crate::core::session::SessionState::new(
                "worktree1".to_string(),
                "para/worktree1".to_string(),
                temp_dir.path().join(".para/worktrees/worktree1"),
            ),
            crate::core::session::SessionState::new_container_with_parent_branch(
                "container1".to_string(),
                "para/container1".to_string(),
                temp_dir.path().join(".para/worktrees/container1"),
                None,
                "main".to_string(),
            ),
            crate::core::session::SessionState::new(
                "worktree2".to_string(),
                "para/worktree2".to_string(),
                temp_dir.path().join(".para/worktrees/worktree2"),
            ),
        ];

        // Save all sessions
        for session in &sessions {
            manager.save_state(session).unwrap();
        }

        // List all sessions
        let loaded_sessions = manager.list_sessions().unwrap();
        assert_eq!(loaded_sessions.len(), 3);

        // Verify session types
        let session_map: std::collections::HashMap<String, SessionType> = loaded_sessions
            .into_iter()
            .map(|s| (s.name.clone(), s.session_type))
            .collect();

        assert_eq!(
            session_map.get("worktree1").unwrap(),
            &SessionType::Worktree
        );
        assert!(matches!(
            session_map.get("container1").unwrap(),
            SessionType::Container { .. }
        ));
        assert_eq!(
            session_map.get("worktree2").unwrap(),
            &SessionType::Worktree
        );
    }

    #[test]
    fn test_cancel_container_session() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();

        let mut manager = SessionManager::new(&config);

        // Create a container session state
        let session = crate::core::session::SessionState::new_container_with_parent_branch(
            "test-cancel".to_string(),
            "para/test-cancel".to_string(),
            temp_dir.path().join(".para/worktrees/test-cancel"),
            None,
            "main".to_string(),
        );

        // Save it
        manager.save_state(&session).unwrap();
        assert!(manager.session_exists(&session.name));

        // Cancel the session (note: this won't actually stop a container since we're just testing state)
        manager.cancel_session(&session.name, false).unwrap();

        // Verify it's removed
        assert!(!manager.session_exists(&session.name));
    }

    #[test]
    fn test_session_type_migration() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let manager = SessionManager::new(&config);

        // Create state directory
        std::fs::create_dir_all(&state_dir).unwrap();

        // Write an old-style session with is_docker field
        let old_session_json = r#"{
            "name": "legacy-session",
            "branch": "para/legacy",
            "worktree_path": "/path/to/worktree",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "task_description": null,
            "last_activity": null,
            "git_stats": null,
            "is_docker": true
        }"#;

        std::fs::write(state_dir.join("legacy-session.state"), old_session_json).unwrap();

        // Load the session - should migrate is_docker to session_type
        let loaded = manager.load_state("legacy-session").unwrap();
        assert!(matches!(loaded.session_type, SessionType::Container { .. }));
        assert_eq!(loaded.is_docker, None);
    }

    #[test]
    fn test_session_persistence_with_type() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Create sessions of both types
        let worktree = crate::core::session::SessionState::new(
            "persist-worktree".to_string(),
            "para/persist-worktree".to_string(),
            temp_dir.path().join(".para/worktrees/persist-worktree"),
        );
        let container = crate::core::session::SessionState::new_container_with_parent_branch(
            "persist-container".to_string(),
            "para/persist-container".to_string(),
            temp_dir.path().join(".para/worktrees/persist-container"),
            Some("abc123".to_string()),
            "main".to_string(),
        );

        // Save and reload sessions
        manager.save_state(&worktree).unwrap();
        manager.save_state(&container).unwrap();

        let loaded_worktree = manager.load_state(&worktree.name).unwrap();
        let loaded_container = manager.load_state(&container.name).unwrap();

        // Verify types are preserved
        assert_eq!(loaded_worktree.session_type, SessionType::Worktree);
        assert!(matches!(
            loaded_container.session_type,
            SessionType::Container { .. }
        ));
        // Verify container has the expected ID
        if let SessionType::Container { container_id } = &loaded_container.session_type {
            assert_eq!(container_id.as_deref(), Some("abc123"));
        } else {
            panic!("Expected container type");
        }
    }

    #[test]
    fn test_create_session_captures_parent_branch() {
        // Create isolated test environment
        let (git_temp, git_service) = setup_test_repo();
        let repo_path = git_service.repository().root.clone();

        // Create .para and state directories first to avoid race conditions
        let para_dir = repo_path.join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a develop branch and switch to it
        Command::new("git")
            .current_dir(&repo_path)
            .args(["checkout", "-b", "develop"])
            .output()
            .expect("Failed to create develop branch");

        // Change to the repo directory for the test
        let _original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_path).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        config.directories.subtrees_dir = ".para/worktrees".to_string();

        let mut manager = SessionManager::new(&config);

        // Create session from develop branch
        let session_state = manager
            .create_session_with_type("test-feature".to_string(), None, None)
            .unwrap();

        // Verify parent branch is captured
        assert_eq!(session_state.parent_branch, Some("develop".to_string()));
        assert_eq!(session_state.name, "test-feature");

        // Reload and verify persistence
        let loaded = manager.load_state("test-feature").unwrap();
        assert_eq!(loaded.parent_branch, Some("develop".to_string()));

        drop(git_temp); // Ensure cleanup
    }

    #[test]
    fn test_create_container_session_captures_parent_branch() {
        // Create isolated test environment
        let (git_temp, git_service) = setup_test_repo();
        let repo_path = git_service.repository().root.clone();

        // Create .para and state directories first
        let para_dir = repo_path.join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a feature/base branch and switch to it
        Command::new("git")
            .current_dir(&repo_path)
            .args(["checkout", "-b", "feature/base"])
            .output()
            .expect("Failed to create feature/base branch");

        // Change to the repo directory for the test
        let _original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_path).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        config.directories.subtrees_dir = ".para/worktrees".to_string();

        let mut manager = SessionManager::new(&config);

        // Create container session from feature/base branch
        let session_state = manager
            .create_session_with_type(
                "test-container".to_string(),
                None,
                Some(SessionType::Container {
                    container_id: Some("container123".to_string()),
                }),
            )
            .unwrap();

        // Verify parent branch is captured for container sessions too
        assert_eq!(
            session_state.parent_branch,
            Some("feature/base".to_string())
        );
        assert_eq!(session_state.name, "test-container");
        assert!(session_state.is_container());

        // Reload and verify persistence
        let loaded = manager.load_state("test-container").unwrap();
        assert_eq!(loaded.parent_branch, Some("feature/base".to_string()));

        drop(git_temp); // Ensure cleanup
    }
}
