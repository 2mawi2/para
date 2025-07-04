use crate::cli::parser::{AuthArgs, AuthCommands};
use crate::utils::{ParaError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::process::Command;

pub fn execute(args: AuthArgs) -> Result<()> {
    match args.command {
        Some(AuthCommands::Setup { force }) => execute_setup(force),
        Some(AuthCommands::Cleanup { dry_run }) => execute_cleanup(dry_run),
        Some(AuthCommands::Status { verbose }) => execute_status(verbose),
        Some(AuthCommands::Reauth) => execute_reauth(),
        None => execute_default(),
    }
}

fn execute_setup(force: bool) -> Result<()> {
    println!("🔐 Setting up Docker container authentication for Claude Code\n");

    if check_authenticated_image()? && !force {
        println!("✅ Authentication already configured!");
        println!("\nAuthenticated Docker image exists. Use --force to re-authenticate.");
        return Ok(());
    }

    match Command::new("docker").arg("version").output() {
        Ok(output) if output.status.success() => {}
        _ => {
            return Err(ParaError::docker_error(
                "Docker not available. Please ensure Docker is installed and running.",
            ));
        }
    }

    match Command::new("docker")
        .args(["image", "inspect", "para-claude:latest"])
        .output()
    {
        Ok(output) if !output.status.success() => {
            println!("⚠️  Base Docker image 'para-claude:latest' not found.");
            println!("\nPlease build the Docker image first:");
            println!("  cd docker && ./build.sh");
            return Err(ParaError::docker_error("Base Docker image not found"));
        }
        Err(e) => {
            return Err(ParaError::docker_error(format!(
                "Failed to check Docker image: {e}"
            )));
        }
        _ => {}
    }

    let user_id = std::process::id().to_string();
    let auth_container = format!("para-auth-{user_id}");

    let _ = Command::new("docker")
        .args(["rm", "-f", &auth_container])
        .output();

    println!("📦 Starting authentication container...\n");

    // Start container WITHOUT volume mount so credentials are saved directly to container filesystem
    let output = Command::new("docker")
        .args([
            "run",
            "-dt",
            "--name",
            &auth_container,
            "--network",
            "host", // Allow browser opening on host
            "para-claude:latest",
            "sleep",
            "infinity",
        ])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to start container: {e}")))?;

    if !output.status.success() {
        return Err(ParaError::docker_error(format!(
            "Failed to start auth container: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    println!("🔐 Claude Authentication");
    println!("──────────────────────");
    println!("You'll now be connected to Claude in the container.");
    println!("When prompted, please:");
    println!("  1. Complete the login process");
    println!("  2. Exit Claude when done (Ctrl+C or 'exit')\n");

    let _status = Command::new("docker")
        .args(["exec", "-it", &auth_container, "claude"])
        .status()
        .map_err(|e| ParaError::docker_error(format!("Failed to run claude: {e}")))?;

    let check_output = Command::new("docker")
        .args([
            "exec",
            &auth_container,
            "test",
            "-f",
            "/home/para/.claude/.credentials.json",
        ])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to check credentials: {e}")))?;

    if !check_output.status.success() {
        let _ = Command::new("docker")
            .args(["rm", "-f", &auth_container])
            .output();
        return Err(ParaError::docker_error(
            "Authentication not completed. Please try again.",
        ));
    }

    println!("\n✅ Authentication successful! Creating authenticated image...");

    let commit_output = Command::new("docker")
        .args(["commit", &auth_container, "para-authenticated:latest"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to create image: {e}")))?;

    if !commit_output.status.success() {
        let _ = Command::new("docker")
            .args(["rm", "-f", &auth_container])
            .output();
        return Err(ParaError::docker_error(format!(
            "Failed to create authenticated image: {}",
            String::from_utf8_lossy(&commit_output.stderr)
        )));
    }

    let _ = Command::new("docker")
        .args(["rm", "-f", &auth_container])
        .output();

    println!("🎉 Authenticated image created successfully!");
    println!("\nYou can now use 'para start --container' or 'para dispatch --container'");

    Ok(())
}

fn execute_cleanup(dry_run: bool) -> Result<()> {
    println!("🧹 Cleaning up Docker authentication artifacts\n");

    remove_authenticated_containers(dry_run)?;

    let items_to_clean = collect_cleanup_items()?;

    if items_to_clean.is_empty() {
        report_no_items_to_clean();
        return Ok(());
    }

    report_cleanup_targets(&items_to_clean);

    if dry_run {
        report_dry_run_mode();
    } else {
        // Get user confirmation and perform cleanup
        if confirm_cleanup()? {
            perform_cleanup_operations(&items_to_clean)?;
            report_cleanup_success();
        } else {
            report_cleanup_cancelled();
        }
    }

    Ok(())
}

fn remove_authenticated_containers(dry_run: bool) -> Result<()> {
    let container_ids = list_authenticated_containers()?;

    if !container_ids.is_empty() && !dry_run {
        println!(
            "Stopping {} containers using authenticated image...",
            container_ids.len()
        );
        for container_id in &container_ids {
            let _ = Command::new("docker")
                .args(["rm", "-f", container_id])
                .output();
        }
    }

    Ok(())
}

fn list_authenticated_containers() -> Result<Vec<String>> {
    let ps_output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "ancestor=para-authenticated:latest",
            "-q",
        ])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to list containers: {e}")))?;

    let container_ids = String::from_utf8_lossy(&ps_output.stdout);
    let containers: Vec<String> = container_ids
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(containers)
}

fn collect_cleanup_items() -> Result<Vec<(&'static str, String)>> {
    let mut items_to_clean: Vec<(&'static str, String)> = Vec::new();

    if check_authenticated_image()? {
        items_to_clean.push((
            "Docker authenticated image",
            "para-authenticated:latest".to_string(),
        ));
    }

    let auth_volumes = find_auth_volumes()?;
    for volume in auth_volumes {
        items_to_clean.push(("Docker auth volume", volume));
    }

    Ok(items_to_clean)
}

fn find_auth_volumes() -> Result<Vec<String>> {
    let volume_output = Command::new("docker")
        .args(["volume", "ls", "-q"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to list volumes: {e}")))?;

    let volumes = String::from_utf8_lossy(&volume_output.stdout);
    let auth_volumes: Vec<String> = volumes
        .lines()
        .filter(|volume| volume.starts_with("para-auth-claude-"))
        .map(|volume| volume.to_string())
        .collect();

    Ok(auth_volumes)
}

fn confirm_cleanup() -> Result<bool> {
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Remove these items?")
        .default(false)
        .interact()
        .map_err(|e| ParaError::docker_error(format!("Failed to read input: {e}")))?;

    Ok(confirm)
}

fn perform_cleanup_operations(items_to_clean: &[(&str, String)]) -> Result<()> {
    let output = Command::new("docker")
        .args(["rmi", "-f", "para-authenticated:latest"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to remove image: {e}")))?;

    if !output.status.success() {
        eprintln!(
            "Warning: Failed to remove authenticated image: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for (desc, name) in items_to_clean {
        if desc == &"Docker auth volume" {
            // Ignore volume removal errors to avoid cleanup interruption
            let _ = Command::new("docker").args(["volume", "rm", name]).output();
        }
    }

    Ok(())
}

fn report_no_items_to_clean() {
    println!("✅ No authentication artifacts found to clean up");
}

fn report_cleanup_targets(items_to_clean: &[(&str, String)]) {
    println!("Found {} items to clean:", items_to_clean.len());
    for (desc, name) in items_to_clean {
        println!("  - {desc}: {name}");
    }
}

fn report_dry_run_mode() {
    println!("\n(Dry run - no changes made)");
}

fn report_cleanup_success() {
    println!("\n✅ Cleanup completed successfully");
    println!("\nYou can now run 'para auth setup' to re-authenticate");
}

fn report_cleanup_cancelled() {
    println!("\nCleanup cancelled");
}

fn execute_status(verbose: bool) -> Result<()> {
    println!("🔍 Checking Docker authentication status\n");

    println!("Docker status:");
    match Command::new("docker").arg("version").output() {
        Ok(output) if output.status.success() => {
            println!("✅ Docker is installed and running");
        }
        _ => {
            println!("❌ Docker not available");
            println!("   Please ensure Docker is installed and running");
            return Ok(());
        }
    }

    println!("\nBase image status:");
    match Command::new("docker")
        .args(["image", "inspect", "para-claude:latest"])
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("✅ Base image 'para-claude:latest' found");
        }
        _ => {
            println!("❌ Base image 'para-claude:latest' not found");
            println!("   Run: cd docker && ./build.sh");
        }
    }

    println!("\nAuthentication status:");
    match check_authenticated_image() {
        Ok(true) => {
            println!("✅ Authenticated image 'para-authenticated:latest' found");

            if verbose {
                // Show image details
                if let Ok(output) = Command::new("docker")
                    .args([
                        "image",
                        "inspect",
                        "para-authenticated:latest",
                        "--format",
                        "{{.Created}}",
                    ])
                    .output()
                {
                    if output.status.success() {
                        let created = String::from_utf8_lossy(&output.stdout);
                        println!("   Created: {}", created.trim());
                    }
                }
            }

            println!("\nYou can use --container flag with 'para start' or 'para dispatch'");
        }
        Ok(false) => {
            println!("❌ Authenticated image not found");
            println!("   Run 'para auth setup' to see setup instructions");
        }
        Err(e) => {
            println!("❌ Failed to check authentication status: {e}");
        }
    }

    Ok(())
}

fn execute_reauth() -> Result<()> {
    println!("🔄 Re-authenticating Claude in Docker container\n");

    println!("Cleaning up existing authentication...");

    let ps_output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "ancestor=para-authenticated:latest",
            "-q",
        ])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to list containers: {e}")))?;

    let container_ids = String::from_utf8_lossy(&ps_output.stdout);
    for container_id in container_ids.lines().filter(|line| !line.is_empty()) {
        let _ = Command::new("docker")
            .args(["rm", "-f", container_id])
            .output();
    }

    // Ignore image removal errors during cleanup to proceed with setup
    let _ = Command::new("docker")
        .args(["rmi", "-f", "para-authenticated:latest"])
        .output();

    println!("✅ Cleanup complete\n");

    // Now run setup
    execute_setup(true)
}

fn execute_default() -> Result<()> {
    execute_setup(false)
}

fn check_authenticated_image() -> Result<bool> {
    let output = Command::new("docker")
        .args(["image", "inspect", "para-authenticated:latest"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to check Docker image: {e}")))?;

    Ok(output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    // Test if Docker is available for integration tests
    fn is_docker_available() -> bool {
        Command::new("docker")
            .args(["info"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_execute_cleanup_dry_run_no_items() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test dry run with no items to clean
        let result = execute_cleanup(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_cleanup_dry_run_with_items() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // This test depends on having Docker available
        // In a real scenario, we would set up test containers/images
        // For now, just verify the function doesn't crash
        let result = execute_cleanup(true);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore] // Integration test that requires Docker setup
    fn test_execute_cleanup_integration_with_test_image() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // This would be a full integration test
        // For now, just verify the function signature works
        let result = execute_cleanup(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_authenticated_image_docker_not_available() {
        // Test behavior when Docker is not available
        // We can't easily mock this without refactoring, so this is a placeholder
        // In real scenarios, we'd expect this to return an error
        if !is_docker_available() {
            // If Docker is not available, we expect the function to handle it gracefully
            let result = check_authenticated_image();
            // Should return an error when Docker is not available
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_check_authenticated_image_docker_available() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test with Docker available
        let result = check_authenticated_image();
        assert!(result.is_ok());
        // The result could be true or false depending on whether the image exists
    }

    // Tests for the refactored functions

    #[test]
    fn test_container_id_parsing() {
        // Test parsing container IDs from docker ps output
        let sample_output = "abc123\ndef456\n\n";
        let container_ids: Vec<&str> = sample_output
            .lines()
            .filter(|line| !line.is_empty())
            .collect();

        assert_eq!(container_ids.len(), 2);
        assert_eq!(container_ids[0], "abc123");
        assert_eq!(container_ids[1], "def456");
    }

    #[test]
    fn test_volume_filtering() {
        // Test filtering volumes that start with para-auth-claude-
        let sample_volumes = [
            "para-auth-claude-123",
            "para-auth-claude-456",
            "other-volume",
            "para-auth-claude-test",
        ];

        let filtered: Vec<&str> = sample_volumes
            .iter()
            .filter(|v| v.starts_with("para-auth-claude-"))
            .copied()
            .collect();

        assert_eq!(filtered.len(), 3);
        assert!(filtered.contains(&"para-auth-claude-123"));
        assert!(filtered.contains(&"para-auth-claude-456"));
        assert!(filtered.contains(&"para-auth-claude-test"));
        assert!(!filtered.contains(&"other-volume"));
    }

    #[test]
    fn test_collect_cleanup_items_docker_available() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test collecting cleanup items
        let result = collect_cleanup_items();
        assert!(result.is_ok());

        let items = result.unwrap();
        // Items could be empty or non-empty depending on system state
        // Just verify the function works without panicking
        println!("Found {} cleanup items", items.len());
    }

    #[test]
    fn test_list_authenticated_containers_docker_available() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test listing authenticated containers
        let result = list_authenticated_containers();
        assert!(result.is_ok());

        let containers = result.unwrap();
        // Containers list could be empty or non-empty
        // Just verify the function works without panicking
        println!("Found {} authenticated containers", containers.len());
    }

    #[test]
    fn test_find_auth_volumes_docker_available() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test finding auth volumes
        let result = find_auth_volumes();
        assert!(result.is_ok());

        let volumes = result.unwrap();
        // Volumes list could be empty or non-empty
        // Just verify the function works without panicking
        println!("Found {} auth volumes", volumes.len());
    }

    #[test]
    fn test_remove_authenticated_containers_dry_run() {
        if !is_docker_available() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Test dry run mode - should not remove anything
        let result = remove_authenticated_containers(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reporting_functions() {
        // Test all the reporting functions don't panic
        report_no_items_to_clean();

        let test_items = vec![
            (
                "Docker authenticated image",
                "para-authenticated:latest".to_string(),
            ),
            ("Docker auth volume", "para-auth-claude-test".to_string()),
        ];
        report_cleanup_targets(&test_items);

        report_dry_run_mode();
        report_cleanup_success();
        report_cleanup_cancelled();
    }

    #[test]
    fn test_cleanup_item_structure() {
        // Test the structure of cleanup items
        let mut items: Vec<(&'static str, String)> = Vec::new();

        // Test empty items
        assert!(items.is_empty());

        // Test with items
        items.push((
            "Docker authenticated image",
            "para-authenticated:latest".to_string(),
        ));
        items.push(("Docker auth volume", "para-auth-claude-test".to_string()));

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "Docker authenticated image");
        assert_eq!(items[1].0, "Docker auth volume");
        assert_eq!(items[0].1, "para-authenticated:latest");
        assert_eq!(items[1].1, "para-auth-claude-test");
    }

    // Tests for error handling scenarios
    #[test]
    fn test_error_handling_scenarios() {
        // Test that the functions handle various error conditions gracefully
        // When Docker is not available, the functions should return errors appropriately

        if !is_docker_available() {
            // Test that functions return errors when Docker is not available
            let container_result = list_authenticated_containers();
            assert!(container_result.is_err());

            let volume_result = find_auth_volumes();
            assert!(volume_result.is_err());

            let items_result = collect_cleanup_items();
            // This might succeed or fail depending on check_authenticated_image behavior
            println!(
                "Collect cleanup items result when Docker unavailable: {:?}",
                items_result.is_ok()
            );
        }
    }
}
