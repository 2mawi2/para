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

    // Check if authenticated image already exists
    if check_authenticated_image()? && !force {
        println!("✅ Authentication already configured!");
        println!("\nAuthenticated Docker image exists. Use --force to re-authenticate.");
        return Ok(());
    }

    // Check Docker availability first
    match Command::new("docker").arg("version").output() {
        Ok(output) if output.status.success() => {}
        _ => {
            return Err(ParaError::docker_error(
                "Docker not available. Please ensure Docker is installed and running.",
            ));
        }
    }

    // Check if base image exists
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

    // Create and run the automated authentication flow
    let user_id = std::process::id().to_string();
    let auth_container = format!("para-auth-{user_id}");

    // Remove any existing auth container
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

    // Run interactive claude session
    let _status = Command::new("docker")
        .args(["exec", "-it", &auth_container, "claude"])
        .status()
        .map_err(|e| ParaError::docker_error(format!("Failed to run claude: {e}")))?;

    // Check if credentials were created by looking in the volume
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
        // Cleanup
        let _ = Command::new("docker")
            .args(["rm", "-f", &auth_container])
            .output();
        return Err(ParaError::docker_error(
            "Authentication not completed. Please try again.",
        ));
    }

    println!("\n✅ Authentication successful! Creating authenticated image...");

    // Create the authenticated image
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

    // Cleanup
    let _ = Command::new("docker")
        .args(["rm", "-f", &auth_container])
        .output();

    println!("🎉 Authenticated image created successfully!");
    println!("\nYou can now use 'para start --container' or 'para dispatch --container'");

    Ok(())
}

fn execute_cleanup(dry_run: bool) -> Result<()> {
    println!("🧹 Cleaning up Docker authentication artifacts\n");

    // First, stop and remove any containers using the authenticated image
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
    let containers_to_remove: Vec<&str> = container_ids
        .lines()
        .filter(|line| !line.is_empty())
        .collect();

    if !containers_to_remove.is_empty() && !dry_run {
        println!(
            "Stopping {} containers using authenticated image...",
            containers_to_remove.len()
        );
        for container_id in &containers_to_remove {
            let _ = Command::new("docker")
                .args(["rm", "-f", container_id])
                .output();
        }
    }

    let mut items_to_clean: Vec<(&str, String)> = Vec::new();

    // Check if authenticated image exists
    if check_authenticated_image()? {
        items_to_clean.push((
            "Docker authenticated image",
            "para-authenticated:latest".to_string(),
        ));
    }

    // Check for auth volumes
    let volume_output = Command::new("docker")
        .args(["volume", "ls", "-q"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to list volumes: {e}")))?;

    let volumes = String::from_utf8_lossy(&volume_output.stdout);
    for volume in volumes.lines() {
        if volume.starts_with("para-auth-claude-") {
            items_to_clean.push(("Docker auth volume", volume.to_string()));
        }
    }

    if items_to_clean.is_empty() {
        println!("✅ No authentication artifacts found to clean up");
        return Ok(());
    }

    println!("Found {} items to clean:", items_to_clean.len());
    for (desc, name) in &items_to_clean {
        println!("  - {desc}: {name}");
    }

    if dry_run {
        println!("\n(Dry run - no changes made)");
    } else {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Remove these items?")
            .default(false)
            .interact()
            .map_err(|e| ParaError::docker_error(format!("Failed to read input: {e}")))?;

        if confirm {
            // Remove the authenticated image (force to handle any issues)
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

            // Remove auth volumes
            for (desc, name) in &items_to_clean {
                if desc == &"Docker auth volume" {
                    let _ = Command::new("docker").args(["volume", "rm", name]).output();
                }
            }

            println!("\n✅ Cleanup completed successfully");
            println!("\nYou can now run 'para auth setup' to re-authenticate");
        } else {
            println!("\nCleanup cancelled");
        }
    }

    Ok(())
}

fn execute_status(verbose: bool) -> Result<()> {
    println!("🔍 Checking Docker authentication status\n");

    // Check Docker availability
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

    // Check base image
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

    // Check authenticated image
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

    // First cleanup (silently, without prompts)
    println!("Cleaning up existing authentication...");

    // Stop and remove any containers using the authenticated image
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

    // Remove the authenticated image
    let _ = Command::new("docker")
        .args(["rmi", "-f", "para-authenticated:latest"])
        .output();

    println!("✅ Cleanup complete\n");

    // Now run setup
    execute_setup(true)
}

fn execute_default() -> Result<()> {
    // Run setup by default for convenience
    execute_setup(false)
}

/// Check if authenticated Docker image exists
fn check_authenticated_image() -> Result<bool> {
    let output = Command::new("docker")
        .args(["image", "inspect", "para-authenticated:latest"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to check Docker image: {e}")))?;

    Ok(output.status.success())
}
