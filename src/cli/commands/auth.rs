use crate::cli::parser::{AuthArgs, AuthCommands};
use crate::utils::{ParaError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::process::Command;

pub fn execute(args: AuthArgs) -> Result<()> {
    match args.command {
        Some(AuthCommands::Setup { force }) => execute_setup(force),
        Some(AuthCommands::Cleanup { dry_run }) => execute_cleanup(dry_run),
        Some(AuthCommands::Status { verbose }) => execute_status(verbose),
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
                "Failed to check Docker image: {}",
                e
            )));
        }
        _ => {}
    }

    println!("📋 Instructions for creating an authenticated Docker image:\n");
    println!("1. Start a temporary container:");
    println!("   docker run -it --name para-auth-temp para-claude:latest bash\n");

    println!("2. Inside the container, authenticate with Claude:");
    println!("   claude /login\n");

    println!("3. After successful authentication, exit the container:");
    println!("   exit\n");

    println!("4. Create the authenticated image:");
    println!("   docker commit para-auth-temp para-authenticated:latest\n");

    println!("5. Clean up the temporary container:");
    println!("   docker rm para-auth-temp\n");

    println!("Once completed, you can use 'para start --container' or 'para dispatch --container'");

    Ok(())
}

fn execute_cleanup(dry_run: bool) -> Result<()> {
    println!("🧹 Cleaning up Docker authentication artifacts\n");

    let mut items_to_clean: Vec<(&str, String)> = Vec::new();

    // Check if authenticated image exists
    if check_authenticated_image()? {
        items_to_clean.push((
            "Docker authenticated image",
            "para-authenticated:latest".to_string(),
        ));
    }

    if items_to_clean.is_empty() {
        println!("✅ No authentication artifacts found to clean up");
        return Ok(());
    }

    println!("Found {} items to clean:", items_to_clean.len());
    for (desc, name) in &items_to_clean {
        println!("  - {}: {}", desc, name);
    }

    if dry_run {
        println!("\n(Dry run - no changes made)");
    } else {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Remove these items?")
            .default(false)
            .interact()
            .map_err(|e| ParaError::docker_error(format!("Failed to read input: {}", e)))?;

        if confirm {
            // Remove the authenticated image
            let output = Command::new("docker")
                .args(["rmi", "para-authenticated:latest"])
                .output()
                .map_err(|e| ParaError::docker_error(format!("Failed to remove image: {}", e)))?;

            if !output.status.success() {
                eprintln!(
                    "Warning: Failed to remove authenticated image: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            println!("\n✅ Cleanup completed successfully");
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
            println!("❌ Failed to check authentication status: {}", e);
        }
    }

    Ok(())
}

fn execute_default() -> Result<()> {
    // Show status by default
    execute_status(false)
}

/// Check if authenticated Docker image exists
fn check_authenticated_image() -> Result<bool> {
    let output = Command::new("docker")
        .args(["image", "inspect", "para-authenticated:latest"])
        .output()
        .map_err(|e| ParaError::docker_error(format!("Failed to check Docker image: {}", e)))?;

    Ok(output.status.success())
}
