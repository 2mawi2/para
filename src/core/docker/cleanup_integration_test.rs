#[cfg(test)]
mod integration_tests {
    use super::super::cleanup::ContainerCleaner;
    use crate::test_utils::test_helpers::*;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    #[ignore] // Ignore by default as it requires Docker
    fn test_cleanup_orphaned_containers_integration() {
        // This test requires Docker to be available
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        // Ensure state directory exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        // Create a test container that looks like an orphaned para container
        let test_container_name = "para-test-orphaned-session";

        // Remove any existing test container
        Command::new("docker")
            .args(["rm", "-f", test_container_name])
            .output()
            .ok();

        // Create a simple test container
        let output = Command::new("docker")
            .args([
                "create",
                "--name",
                test_container_name,
                "busybox:latest",
                "sleep",
                "infinity",
            ])
            .output()
            .expect("Failed to create test container");

        assert!(output.status.success(), "Failed to create test container");

        // Verify container exists
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", test_container_name),
                "--format",
                "{{.Names}}",
            ])
            .output()
            .unwrap();

        let container_list = String::from_utf8_lossy(&output.stdout);
        assert!(container_list.contains(test_container_name));

        // Run cleanup
        let cleaner = ContainerCleaner::new(config);
        cleaner.cleanup_orphaned_containers().unwrap();

        // Verify container was removed
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", test_container_name),
                "--format",
                "{{.Names}}",
            ])
            .output()
            .unwrap();

        let container_list = String::from_utf8_lossy(&output.stdout);
        assert!(
            !container_list.contains(test_container_name),
            "Container should have been removed"
        );
    }

    #[test]
    #[ignore] // Ignore by default as it requires Docker
    fn test_cleanup_preserves_active_sessions() {
        // This test requires Docker to be available
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        // Ensure state directory exists
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        // Create a test container
        let session_name = "test-active-session";
        let test_container_name = format!("para-{}", session_name);

        // Remove any existing test container
        Command::new("docker")
            .args(["rm", "-f", &test_container_name])
            .output()
            .ok();

        // Create a simple test container
        Command::new("docker")
            .args([
                "create",
                "--name",
                &test_container_name,
                "busybox:latest",
                "sleep",
                "infinity",
            ])
            .output()
            .expect("Failed to create test container");

        // Create a state file for this session
        let state_file = temp_dir.path().join(format!("{}.state", session_name));
        std::fs::write(&state_file, "{}").unwrap();

        // Run cleanup
        let cleaner = ContainerCleaner::new(config);
        cleaner.cleanup_orphaned_containers().unwrap();

        // Verify container still exists
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", test_container_name),
                "--format",
                "{{.Names}}",
            ])
            .output()
            .unwrap();

        let container_list = String::from_utf8_lossy(&output.stdout);
        assert!(
            container_list.contains(&test_container_name),
            "Container should NOT have been removed"
        );

        // Cleanup
        Command::new("docker")
            .args(["rm", "-f", &test_container_name])
            .output()
            .ok();
    }

    fn is_docker_available() -> bool {
        Command::new("docker")
            .args(["info"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}
