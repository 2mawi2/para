#[cfg(test)]
mod tests {
    use super::super::launcher::{wrap_command_with_sandbox, SandboxOptions};
    use tempfile::TempDir;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_sandbox_allows_var_folders_temp_files() {
        // Create a test worktree directory
        let worktree = TempDir::new().unwrap();

        // Get the actual temp directory to build realistic test paths
        let temp_dir = std::env::temp_dir();
        let temp_str = temp_dir.to_string_lossy();

        // Test various temp directory patterns that tools might use
        let test_paths = vec![
            format!("{}/just-test", temp_str),
            format!("{}/temp-file-12345", temp_str),
            format!("{}/some-tool-temp", temp_str),
            "/tmp/test-file".to_string(),
        ];

        let options = SandboxOptions {
            profile: "standard".to_string(),
            proxy_address: None,
            allowed_domains: vec![],
        };

        for test_path in test_paths {
            // Create a command that tries to write to the temp path
            let command = format!("touch '{test_path}'");

            let result = wrap_command_with_sandbox(&command, worktree.path(), &options);

            assert!(
                result.is_ok(),
                "Failed to wrap command for path: {test_path}"
            );

            let sandboxed = result.unwrap();

            // Verify the command includes sandbox-exec on macOS
            assert!(
                sandboxed.command.contains("sandbox-exec"),
                "Command should contain sandbox-exec"
            );

            // The command gets escaped, so we need to check for the escaped version
            let escaped_command = command.replace('\'', "'\\''");
            assert!(
                sandboxed.command.contains(&escaped_command),
                "Command should contain escaped command. Looking for: '{escaped_command}' in '{}'",
                sandboxed.command
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_just_exact_temp_file_pattern() {
        use std::process::Command;

        // This test reproduces the exact error from Just
        let worktree = TempDir::new().unwrap();

        let options = SandboxOptions {
            profile: "standard".to_string(),
            proxy_address: None,
            allowed_domains: vec![],
        };

        // Use the actual system temp directory instead of hardcoded path
        let temp_dir = std::env::temp_dir();
        let just_temp_path = temp_dir.join(format!("just-test-{}", std::process::id()));

        // Test if we can create a file at this path (simulating Just's behavior)
        let command = format!(
            "mkdir -p '{}' && echo 'test' > '{}/test.txt' && rm -rf '{}'",
            just_temp_path.display(),
            just_temp_path.display(),
            just_temp_path.display()
        );
        let wrapped = wrap_command_with_sandbox(&command, worktree.path(), &options).unwrap();

        // Try to execute it
        let output = Command::new("sh").arg("-c").arg(&wrapped.command).output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() {
                    panic!(
                        "Failed to create Just temp file. This means our sandbox profile is still too restrictive.\nstderr: {stderr}\nstdout: {stdout}"
                    );
                }
            }
            Err(e) => {
                panic!("Failed to execute command: {e}");
            }
        }
    }

    #[test]
    fn test_sandbox_does_nothing_on_non_macos() {
        #[cfg(not(target_os = "macos"))]
        {
            let worktree = TempDir::new().unwrap();

            let options = SandboxOptions {
                profile: "standard".to_string(),
                proxy_address: None,
                allowed_domains: vec![],
            };

            let command = "echo 'test'";
            let result = wrap_command_with_sandbox(command, worktree.path(), &options).unwrap();

            // On non-macOS, command should be unchanged
            assert_eq!(result.command, command);
            assert!(!result.needs_wrapper_script);
        }
    }
}
