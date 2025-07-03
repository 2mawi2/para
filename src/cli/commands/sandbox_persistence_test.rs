#[cfg(test)]
mod tests {
    use crate::core::session::SessionManager;
    use crate::test_utils::test_helpers::create_test_config;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_persistence_in_start_command() {
        // Create a simple session state without actual git operations
        let session_name = format!("test-sandbox-{}", uuid::Uuid::new_v4());
        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session = crate::core::session::state::SessionState::with_all_flags(
            session_name.clone(),
            format!("para/{session_name}"),
            worktree_path,
            "main".to_string(),
            false,
            true,
            Some("permissive".to_string()),
        );

        // Verify sandbox settings are saved
        assert_eq!(session.sandbox_enabled, Some(true));
        assert_eq!(session.sandbox_profile, Some("permissive".to_string()));

        // Simulate loading by creating test config and session manager
        let test_config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = test_config;
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);
        session_manager.save_state(&session).unwrap();

        // Load the session and verify sandbox settings persist
        let loaded_session = session_manager.load_state(&session_name).unwrap();
        assert_eq!(loaded_session.sandbox_enabled, Some(true));
        assert_eq!(
            loaded_session.sandbox_profile,
            Some("permissive".to_string())
        );
    }

    #[test]
    fn test_sandbox_persistence_in_dispatch_command() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Simulate dispatch with restrictive sandbox
        let session_state = crate::core::session::state::SessionState::with_all_flags(
            "dispatch-sandbox".to_string(),
            "para/dispatch-sandbox".to_string(),
            temp_dir.path().join("dispatch-sandbox"),
            "main".to_string(),
            false,
            true,
            Some("restrictive".to_string()),
        );

        // Save the session
        let session_manager = SessionManager::new(&config);
        session_manager.save_state(&session_state).unwrap();

        // Load and verify
        let loaded_session = session_manager.load_state("dispatch-sandbox").unwrap();
        assert_eq!(loaded_session.sandbox_enabled, Some(true));
        assert_eq!(
            loaded_session.sandbox_profile,
            Some("restrictive".to_string())
        );
    }

    #[test]
    fn test_sandbox_persistence_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create old session file without sandbox fields
        let worktree_path = temp_dir.path().join("old-worktree");
        let old_session_json = format!(
            r#"{{
            "name": "old-session",
            "branch": "para/old-session",
            "worktree_path": "{}",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree"
        }}"#,
            worktree_path.display()
        );

        let session_file = state_dir.join("old-session.state");
        std::fs::write(&session_file, old_session_json).unwrap();

        // Load old session
        let session_manager = SessionManager::new(&config);
        let loaded_session = session_manager.load_state("old-session").unwrap();

        // Should have None for sandbox fields
        assert_eq!(loaded_session.sandbox_enabled, None);
        assert_eq!(loaded_session.sandbox_profile, None);
    }

    #[test]
    fn test_resume_uses_stored_sandbox_settings() {
        let temp_dir = TempDir::new().unwrap();
        let session_name = format!("resume-test-{}", uuid::Uuid::new_v4());

        let mut config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a session with sandbox enabled
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session = crate::core::session::state::SessionState::with_all_flags(
            session_name.clone(),
            format!("para/{session_name}"),
            worktree_path,
            "main".to_string(),
            false,
            true,
            Some("restrictive".to_string()),
        );

        // Save the session state
        let session_manager = SessionManager::new(&config);
        session_manager.save_state(&session).unwrap();

        // Simulate resume without sandbox args (should use stored settings)
        let loaded_session = session_manager.load_state(&session_name).unwrap();
        assert_eq!(loaded_session.sandbox_enabled, Some(true));
        assert_eq!(
            loaded_session.sandbox_profile,
            Some("restrictive".to_string())
        );
    }

    #[test]
    fn test_cli_override_stored_sandbox_settings() {
        let temp_dir = TempDir::new().unwrap();
        let session_name = format!("override-test-{}", uuid::Uuid::new_v4());

        let mut config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a session with sandbox enabled
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session = crate::core::session::state::SessionState::with_all_flags(
            session_name.clone(),
            format!("para/{session_name}"),
            worktree_path,
            "main".to_string(),
            false,
            true,
            Some("permissive".to_string()),
        );

        // Verify initial settings
        assert_eq!(session.sandbox_enabled, Some(true));
        assert_eq!(session.sandbox_profile, Some("permissive".to_string()));

        // Save and reload to verify persistence
        let session_manager = SessionManager::new(&config);
        session_manager.save_state(&session).unwrap();

        // CLI args should override stored settings when resuming
        // This is handled in the resume command logic, not in the session state
        // The test verifies that the data is properly stored and can be retrieved
        let loaded_session = session_manager.load_state(&session_name).unwrap();
        assert_eq!(loaded_session.sandbox_enabled, Some(true));
        assert_eq!(
            loaded_session.sandbox_profile,
            Some("permissive".to_string())
        );
    }

    #[test]
    fn test_dangerous_skip_permissions_with_sandbox() {
        let temp_dir = TempDir::new().unwrap();
        let session_name = format!("danger-sandbox-test-{}", uuid::Uuid::new_v4());

        let mut config = create_test_config();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a session with both sandbox and dangerous_skip_permissions enabled
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session = crate::core::session::state::SessionState::with_all_flags(
            session_name.clone(),
            format!("para/{session_name}"),
            worktree_path,
            "main".to_string(),
            true, // dangerous_skip_permissions enabled
            true, // sandbox enabled
            Some("permissive".to_string()),
        );

        // Verify both settings are saved
        assert_eq!(session.dangerous_skip_permissions, Some(true));
        assert_eq!(session.sandbox_enabled, Some(true));
        assert_eq!(session.sandbox_profile, Some("permissive".to_string()));

        // Save and reload to verify persistence
        let session_manager = SessionManager::new(&config);
        session_manager.save_state(&session).unwrap();

        // Load and verify both settings persist
        let loaded_session = session_manager.load_state(&session_name).unwrap();
        assert_eq!(loaded_session.dangerous_skip_permissions, Some(true));
        assert_eq!(loaded_session.sandbox_enabled, Some(true));
        assert_eq!(
            loaded_session.sandbox_profile,
            Some("permissive".to_string())
        );
    }
}
