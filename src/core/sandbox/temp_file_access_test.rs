#[cfg(test)]
mod tests {
    use super::super::launcher::{wrap_command_with_sandbox, SandboxOptions};
    use tempfile::TempDir;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_sandbox_allows_var_folders_temp_files() {
        // Create a test worktree directory
        let worktree = TempDir::new().unwrap();

        // Test various /var/folders patterns that tools might use
        let test_paths = vec![
            "/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/just-xKAsgP",
            "/var/folders/xy/abc123def456/T/temp-file-12345",
            "/var/folders/a/b/c/some-tool-temp",
            "/var/folders/nested/deep/path/to/temp/file.tmp",
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

        // The exact path that Just is trying to create
        let just_temp_path = "/var/folders/bl/5pq1htk93_v7n8zv5wsxnzqm0000gn/T/just-ZmK2Hp";

        // Test if we can create a file at this exact path
        let command =
            format!("mkdir -p '{just_temp_path}' && echo 'test' > '{just_temp_path}/test.txt'");
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
