use anyhow::Result;
use std::path::Path;

/// Options for sandbox wrapping
#[derive(Debug, Clone)]
pub struct SandboxOptions {
    pub profile: String,
    pub proxy_address: Option<String>,
    /// Domains to allow through the network proxy (used by callers when generating wrapper scripts)
    pub allowed_domains: Vec<String>,
}

/// Result of sandbox wrapping that includes the command and any cleanup needed
#[derive(Debug)]
pub struct SandboxedCommand {
    /// The full command to execute
    pub command: String,
    /// Whether this needs special handling (e.g., network proxy)
    pub needs_wrapper_script: bool,
    /// Port for network proxy if applicable
    pub proxy_port: Option<u16>,
    /// Domains to allow through the network proxy
    pub allowed_domains: Vec<String>,
}

/// Wraps a command with macOS sandbox-exec if sandboxing is enabled
/// This new version returns structured information instead of just a string
pub fn wrap_command_with_sandbox(
    command: &str,
    worktree_path: &Path,
    options: &SandboxOptions,
) -> Result<SandboxedCommand> {
    // Validate profile name on all platforms for consistency
    if options.profile.is_empty() {
        return Err(anyhow::anyhow!("Sandbox profile name cannot be empty"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On non-macOS, return the command unchanged
        // Note: proxy_address is not used on non-macOS platforms but we access it to avoid dead code warnings
        let _proxy_port: Option<u16> = options
            .proxy_address
            .as_ref()
            .and_then(|addr| addr.split(':').nth(1)?.parse().ok());

        // Suppress unused parameter warning for non-macOS
        let _ = worktree_path;

        Ok(SandboxedCommand {
            command: command.to_string(),
            needs_wrapper_script: false,
            proxy_port: None,
            allowed_domains: options.allowed_domains.clone(),
        })
    }

    #[cfg(target_os = "macos")]
    {
        use super::profiles::extract_profile;
        use anyhow::Context;

        let profile_path =
            extract_profile(&options.profile).context("Failed to extract sandbox profile")?;

        // Get required directories
        let home_dir = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine user directories"))?
            .home_dir()
            .to_path_buf();

        let cache_dir = directories::ProjectDirs::from("", "", "para")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| home_dir.join("Library/Caches"));

        let temp_dir = std::env::temp_dir();

        // Remove trailing slashes from paths
        let temp_dir_str = temp_dir.to_string_lossy().trim_end_matches('/').to_string();
        let home_dir_str = home_dir.to_string_lossy().trim_end_matches('/').to_string();
        let cache_dir_str = cache_dir
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();
        let worktree_path_str = worktree_path
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string();

        // Escape the command for shell
        let escaped_command = command.replace('\'', "'\\''");

        // Get the main repository directory (parent of .para/worktrees)
        let main_repo_dir = if worktree_path_str.contains("/.para/worktrees/") {
            // Extract main repo from worktree path
            worktree_path_str
                .split("/.para/worktrees/")
                .next()
                .unwrap_or(&worktree_path_str)
                .to_string()
        } else {
            // If not in a worktree, use the current path as main repo
            worktree_path_str.clone()
        };

        // Build the sandbox-exec command
        let mut sandbox_cmd = format!(
            "sandbox-exec \
             -D 'TARGET_DIR={worktree_path_str}' \
             -D 'TMP_DIR={temp_dir_str}' \
             -D 'HOME_DIR={home_dir_str}' \
             -D 'CACHE_DIR={cache_dir_str}' \
             -D 'MAIN_REPO_DIR={main_repo_dir}'"
        );

        // Add proxy address parameter if provided
        if let Some(ref proxy_addr) = options.proxy_address {
            sandbox_cmd.push_str(&format!(" -D 'PROXY_ADDR={proxy_addr}'"));
        }

        // Add the profile and command
        sandbox_cmd.push_str(&format!(
            " -f '{}' sh -c '{}'",
            profile_path.display(),
            escaped_command
        ));

        // Determine if we need special handling for network proxy
        let (needs_wrapper, proxy_port) = if options.proxy_address.is_some() {
            let port = options
                .proxy_address
                .as_ref()
                .and_then(|addr| addr.split(':').next_back())
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8877);
            (true, Some(port))
        } else {
            (false, None)
        };

        Ok(SandboxedCommand {
            command: sandbox_cmd,
            needs_wrapper_script: needs_wrapper,
            proxy_port,
            allowed_domains: options.allowed_domains.clone(),
        })
    }
}

/// Generate a wrapper script for network-sandboxed commands
/// This runs the proxy outside the sandbox and the command inside
pub fn generate_network_sandbox_wrapper(
    sandboxed_command: &str,
    proxy_port: u16,
    allowed_domains: &[String],
) -> String {
    let domains_arg = if allowed_domains.is_empty() {
        String::new()
    } else {
        format!(" --allowed-domains '{}'", allowed_domains.join(","))
    };

    format!(
        r#"#!/bin/bash
# Para network sandboxing wrapper
# This script manages proxy lifecycle and sandbox execution

echo "🚀 Starting Claude with network sandboxing..."

# Save the script path for self-deletion
SCRIPT_PATH="$0"

# Start proxy in background (OUTSIDE sandbox)
echo "🌐 Starting network proxy on port {proxy_port}..."
para proxy --port {proxy_port}{domains_arg} >/dev/null 2>&1 &
PROXY_PID=$!

# Function to cleanup
cleanup() {{
    echo "🛑 Stopping proxy..."
    kill $PROXY_PID 2>/dev/null || true
    # Self-delete this script
    rm -f "$SCRIPT_PATH" 2>/dev/null || true
}}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Wait for proxy to be ready
echo "⏳ Waiting for proxy to be ready..."
for i in {{1..40}}; do
    if nc -z 127.0.0.1 {proxy_port} 2>/dev/null; then
        echo "✅ Proxy is ready"
        break
    fi
    if [ $i -eq 40 ]; then
        echo "❌ Proxy failed to start"
        exit 1
    fi
    sleep 0.25
done

# Export proxy environment variables and run the sandboxed command
export HTTP_PROXY=http://127.0.0.1:{proxy_port}
export HTTPS_PROXY=http://127.0.0.1:{proxy_port}
export NO_PROXY=localhost,127.0.0.1

# Run the actual command INSIDE the sandbox
{sandboxed_command}
STATUS=$?

# Cleanup happens via trap
exit $STATUS"#
    )
}

/// Check if sandbox-exec is available on the system
pub fn is_sandbox_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("which")
            .arg("sandbox-exec")
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_options_creation() {
        let options = SandboxOptions {
            profile: "standard".to_string(),
            proxy_address: None,
            allowed_domains: vec![],
        };
        assert_eq!(options.profile, "standard");
        assert!(options.proxy_address.is_none());
        assert!(options.allowed_domains.is_empty());
    }

    #[test]
    fn test_sandboxed_command_structure() {
        let cmd = SandboxedCommand {
            command: "test command".to_string(),
            needs_wrapper_script: false,
            proxy_port: None,
            allowed_domains: vec![],
        };
        assert_eq!(cmd.command, "test command");
        assert!(!cmd.needs_wrapper_script);
        assert!(cmd.proxy_port.is_none());
        assert!(cmd.allowed_domains.is_empty());
    }

    #[test]
    fn test_generate_network_sandbox_wrapper() {
        let wrapper = generate_network_sandbox_wrapper(
            "sandbox-exec -f profile.sb sh -c 'claude'",
            8877,
            &["example.com".to_string()],
        );

        assert!(wrapper.contains("#!/bin/bash"));
        assert!(wrapper.contains("para proxy --port 8877"));
        assert!(wrapper.contains("--allowed-domains 'example.com'"));
        assert!(wrapper.contains("export HTTP_PROXY=http://127.0.0.1:8877"));
        assert!(wrapper.contains("trap cleanup EXIT INT TERM"));
        assert!(wrapper.contains("sandbox-exec -f profile.sb sh -c 'claude'"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_wrap_command_with_sandbox_macos() {
        let temp_dir = TempDir::new().unwrap();
        let options = SandboxOptions {
            profile: "standard".to_string(),
            proxy_address: None,
            allowed_domains: vec![],
        };

        let result = wrap_command_with_sandbox("echo hello", temp_dir.path(), &options);
        assert!(result.is_ok());

        let sandboxed = result.unwrap();
        assert!(sandboxed.command.contains("sandbox-exec"));
        assert!(sandboxed.command.contains("echo hello"));
        assert!(!sandboxed.needs_wrapper_script);
        assert!(sandboxed.proxy_port.is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_wrap_command_with_network_sandbox() {
        let temp_dir = TempDir::new().unwrap();
        let options = SandboxOptions {
            profile: "standard-proxied".to_string(),
            proxy_address: Some("127.0.0.1:8877".to_string()),
            allowed_domains: vec!["example.com".to_string()],
        };

        let result = wrap_command_with_sandbox("claude", temp_dir.path(), &options);
        assert!(result.is_ok());

        let sandboxed = result.unwrap();
        assert!(sandboxed.command.contains("sandbox-exec"));
        assert!(sandboxed.command.contains("-D 'PROXY_ADDR=127.0.0.1:8877'"));
        assert!(sandboxed.needs_wrapper_script);
        assert_eq!(sandboxed.proxy_port, Some(8877));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_wrap_command_non_macos() {
        let temp_dir = TempDir::new().unwrap();
        let options = SandboxOptions {
            profile: "standard".to_string(),
            proxy_address: None,
            allowed_domains: vec![],
        };

        let result = wrap_command_with_sandbox("echo hello", temp_dir.path(), &options);
        assert!(result.is_ok());

        let sandboxed = result.unwrap();
        assert_eq!(sandboxed.command, "echo hello");
        assert!(!sandboxed.needs_wrapper_script);
        assert!(sandboxed.proxy_port.is_none());
    }
}
