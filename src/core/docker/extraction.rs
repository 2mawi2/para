//! Container code extraction for applying changes from containers to host Git branches

use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::core::git::{GitOperations, GitRepository};

use super::{
    error::{DockerError, DockerResult},
    service::DockerService,
};

/// Progress callback for long-running operations
pub type ProgressCallback = Box<dyn Fn(&str) + Send>;

/// Options for extracting container changes
pub struct ExtractionOptions {
    /// The session/container name
    pub session_name: String,
    /// The commit message for the changes
    pub commit_message: String,
    /// The target branch name (optional, will generate if not provided)
    pub target_branch: Option<String>,
    /// Whether to handle binary files
    pub include_binary: bool,
    /// Whether to preserve file permissions
    pub preserve_permissions: bool,
    /// Progress callback for reporting status
    pub progress_callback: Option<ProgressCallback>,
}

/// Result of container extraction
#[derive(Debug)]
pub struct ExtractionResult {
    /// The branch created with the changes
    pub branch_name: String,
    /// The commit SHA created
    #[allow(dead_code)]
    pub commit_sha: String,
    /// Number of files changed
    pub files_changed: usize,
    /// Number of lines added
    pub lines_added: usize,
    /// Number of lines removed
    pub lines_removed: usize,
}

/// Container code extractor
pub struct ContainerExtractor<'a> {
    docker_service: &'a dyn DockerService,
    git_repo: GitRepository,
}

impl<'a> ContainerExtractor<'a> {
    /// Create a new container extractor
    pub fn new(docker_service: &'a dyn DockerService, git_repo: GitRepository) -> Self {
        Self {
            docker_service,
            git_repo,
        }
    }

    /// Extract changes from a container and apply them to a new Git branch
    pub fn extract_container_changes(
        &self,
        options: ExtractionOptions,
    ) -> DockerResult<ExtractionResult> {
        // Validate container is running
        let status = self
            .docker_service
            .get_container_status(&options.session_name)?;
        if !matches!(status, super::session::ContainerStatus::Running) {
            return Err(DockerError::ContainerNotRunning {
                name: options.session_name.clone(),
            });
        }

        // Create a temporary directory for patch files
        let temp_dir = std::env::temp_dir().join(format!("para-extract-{}", &options.session_name));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to create temp dir: {}", e)))?;
        let patch_path = temp_dir.join("changes.patch");

        self.report_progress(&options, "Preparing container for extraction...");

        // Stage 1: Commit changes in the container
        self.commit_container_changes(&options)?;

        // Stage 2: Generate patch using git format-patch
        let patch_content = self.generate_patch_in_container(&options)?;

        // Stage 3: Save patch to temporary file
        std::fs::write(&patch_path, &patch_content).map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Failed to write patch file: {}", e))
        })?;

        // Stage 4: Apply patch to host repository
        let result = self.apply_patch_to_host(&options, &patch_path)?;

        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&temp_dir);

        self.report_progress(&options, "Container changes extracted successfully");

        Ok(result)
    }

    /// Commit all changes inside the container
    fn commit_container_changes(&self, options: &ExtractionOptions) -> DockerResult<()> {
        self.report_progress(options, "Staging changes in container...");

        // Stage all changes
        let output = self.docker_service.exec_in_container(
            &options.session_name,
            "git",
            &["add".to_string(), "-A".to_string()],
            None,
        )?;

        if output.contains("fatal:") {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to stage changes in container: {}",
                output
            )));
        }

        // Check if there are changes to commit
        let status_output = self.docker_service.exec_in_container(
            &options.session_name,
            "git",
            &["status".to_string(), "--porcelain".to_string()],
            None,
        )?;

        if status_output.trim().is_empty() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "No changes to extract from container"
            )));
        }

        self.report_progress(options, "Committing changes in container...");

        // Commit the changes
        let commit_output = self.docker_service.exec_in_container(
            &options.session_name,
            "git",
            &[
                "commit".to_string(),
                "-m".to_string(),
                options.commit_message.clone(),
            ],
            None,
        )?;

        if commit_output.contains("fatal:") || commit_output.contains("error:") {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to commit changes in container: {}",
                commit_output
            )));
        }

        Ok(())
    }

    /// Generate a patch file using git format-patch in the container
    fn generate_patch_in_container(&self, options: &ExtractionOptions) -> DockerResult<Vec<u8>> {
        self.report_progress(options, "Generating patch from container changes...");

        // Get the current branch in the container (for future use)
        let branch_output = self.docker_service.exec_in_container(
            &options.session_name,
            "git",
            &[
                "rev-parse".to_string(),
                "--abbrev-ref".to_string(),
                "HEAD".to_string(),
            ],
            None,
        )?;
        let _container_branch = branch_output.trim();

        // Generate patch for the last commit
        let mut patch_args = vec![
            "format-patch".to_string(),
            "-1".to_string(), // Last commit only
            "--stdout".to_string(),
            "--no-stat".to_string(),
        ];

        if options.include_binary {
            patch_args.push("--binary".to_string());
        }

        if options.preserve_permissions {
            // Git format-patch preserves permissions by default
        }

        let patch_output = self.docker_service.exec_in_container(
            &options.session_name,
            "git",
            &patch_args,
            None,
        )?;

        if patch_output.is_empty() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "No patch generated from container"
            )));
        }

        Ok(patch_output.into_bytes())
    }

    /// Apply the patch to the host repository with atomic rollback support
    fn apply_patch_to_host(
        &self,
        options: &ExtractionOptions,
        patch_path: &Path,
    ) -> DockerResult<ExtractionResult> {
        self.report_progress(options, "Creating target branch on host...");

        // Get current branch for rollback
        let original_branch = self.git_repo.get_current_branch().map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Failed to get current branch: {}", e))
        })?;

        // Generate target branch name if not provided
        let target_branch = options
            .target_branch
            .clone()
            .unwrap_or_else(|| format!("container/{}", options.session_name));

        // Check if target branch already exists
        let branch_exists = self.git_repo.branch_exists(&target_branch).map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Failed to check branch existence: {}", e))
        })?;

        if branch_exists {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Target branch '{}' already exists. Please choose a different branch name or delete the existing branch.",
                target_branch
            )));
        }

        // Create new branch
        if let Err(e) = self
            .git_repo
            .create_branch(&target_branch, &original_branch)
        {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to create branch: {}",
                e
            )));
        }

        // Checkout the new branch - rollback on failure
        if let Err(e) = self.git_repo.checkout_branch(&target_branch) {
            // Rollback: delete the created branch
            let _ = self.git_repo.delete_branch(&target_branch, true);
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to checkout branch: {}",
                e
            )));
        }

        self.report_progress(options, "Applying patch to host repository...");

        // Apply the patch using git apply - rollback on failure
        let apply_result = match self.apply_patch_with_git(patch_path) {
            Ok(result) => result,
            Err(e) => {
                // Rollback: switch back to original branch and delete target branch
                let _ = self.git_repo.checkout_branch(&original_branch);
                let _ = self.git_repo.delete_branch(&target_branch, true);
                return Err(e);
            }
        };

        // Parse the patch to get statistics
        let stats = match self.parse_patch_stats(patch_path) {
            Ok(stats) => stats,
            Err(_) => {
                // Even if stats parsing fails, we've successfully applied the patch
                // So we'll just return default stats
                PatchStats {
                    files_changed: 0,
                    lines_added: 0,
                    lines_removed: 0,
                }
            }
        };

        // Checkout back to original branch
        if let Err(e) = self.git_repo.checkout_branch(&original_branch) {
            // This is non-critical - the patch was applied successfully
            self.report_progress(
                options,
                &format!("Warning: Could not switch back to original branch: {}", e),
            );
        }

        Ok(ExtractionResult {
            branch_name: target_branch,
            commit_sha: apply_result.commit_sha,
            files_changed: stats.files_changed,
            lines_added: stats.lines_added,
            lines_removed: stats.lines_removed,
        })
    }

    /// Apply patch using git command-line tool
    fn apply_patch_with_git(&self, patch_path: &Path) -> DockerResult<ApplyResult> {
        use std::process::Command;

        // First, try to apply the patch to check for conflicts
        let check_output = Command::new("git")
            .args(["apply", "--check", patch_path.to_str().unwrap()])
            .current_dir(&self.git_repo.root)
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to run git apply --check: {}", e))
            })?;

        if !check_output.status.success() {
            let error = String::from_utf8_lossy(&check_output.stderr);
            return Err(DockerError::Other(anyhow::anyhow!(
                "Patch conflicts detected: {}",
                error
            )));
        }

        // Apply the patch
        let apply_output = Command::new("git")
            .args(["am", "--3way", patch_path.to_str().unwrap()])
            .current_dir(&self.git_repo.root)
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to run git am: {}", e)))?;

        if !apply_output.status.success() {
            let error = String::from_utf8_lossy(&apply_output.stderr);

            // Try to abort the failed am operation
            let _ = Command::new("git")
                .args(["am", "--abort"])
                .current_dir(&self.git_repo.root)
                .output();

            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to apply patch: {}",
                error
            )));
        }

        // Get the commit SHA
        let commit_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.git_repo.root)
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to get commit SHA: {}", e)))?;

        let commit_sha = String::from_utf8_lossy(&commit_output.stdout)
            .trim()
            .to_string();

        Ok(ApplyResult { commit_sha })
    }

    /// Parse patch file to extract statistics
    fn parse_patch_stats(&self, patch_path: &Path) -> DockerResult<PatchStats> {
        let file = std::fs::File::open(patch_path)
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to open patch file: {}", e)))?;
        let reader = BufReader::new(file);

        let mut files_changed = 0;
        let mut lines_added = 0;
        let mut lines_removed = 0;
        let mut in_diff = false;

        for line in reader.lines() {
            let line = line.map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to read patch line: {}", e))
            })?;

            if line.starts_with("diff --git") {
                files_changed += 1;
                in_diff = true;
            } else if in_diff {
                if line.starts_with('+') && !line.starts_with("+++") {
                    lines_added += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    lines_removed += 1;
                }
            }
        }

        Ok(PatchStats {
            files_changed,
            lines_added,
            lines_removed,
        })
    }

    /// Report progress if a callback is provided
    fn report_progress(&self, options: &ExtractionOptions, message: &str) {
        if let Some(ref callback) = options.progress_callback {
            callback(message);
        }
    }
}

/// Result of applying a patch
struct ApplyResult {
    commit_sha: String,
}

/// Statistics from a patch file
struct PatchStats {
    files_changed: usize,
    lines_added: usize,
    lines_removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::docker::service::MockDockerService;
    use crate::test_utils::test_helpers::*;

    fn create_test_extractor() -> (::tempfile::TempDir, ContainerExtractor<'static>) {
        let (_git_temp, git_repo) = setup_test_repo();
        let docker_service = Box::new(MockDockerService);
        let docker_service_ref: &'static MockDockerService = Box::leak(docker_service);
        let extractor = ContainerExtractor::new(docker_service_ref, git_repo.repository().clone());
        (_git_temp, extractor)
    }

    #[test]
    fn test_extraction_options_creation() {
        let options = ExtractionOptions {
            session_name: "test-session".to_string(),
            commit_message: "Test commit".to_string(),
            target_branch: Some("feature/test".to_string()),
            include_binary: true,
            preserve_permissions: true,
            progress_callback: None,
        };

        assert_eq!(options.session_name, "test-session");
        assert_eq!(options.commit_message, "Test commit");
        assert_eq!(options.target_branch, Some("feature/test".to_string()));
        assert!(options.include_binary);
        assert!(options.preserve_permissions);
    }

    #[test]
    fn test_extraction_result_creation() {
        let result = ExtractionResult {
            branch_name: "feature/extracted".to_string(),
            commit_sha: "abc123".to_string(),
            files_changed: 5,
            lines_added: 100,
            lines_removed: 50,
        };

        assert_eq!(result.branch_name, "feature/extracted");
        assert_eq!(result.commit_sha, "abc123");
        assert_eq!(result.files_changed, 5);
        assert_eq!(result.lines_added, 100);
        assert_eq!(result.lines_removed, 50);
    }

    #[test]
    fn test_parse_patch_stats() {
        let temp_dir = ::tempfile::TempDir::new().unwrap();
        let patch_path = temp_dir.path().join("test.patch");

        // Create a sample patch file
        let patch_content = r#"From abc123 Mon Sep 17 00:00:00 2001
From: Test User <test@example.com>
Date: Mon, 17 Sep 2023 12:00:00 +0000
Subject: [PATCH] Test commit

diff --git a/file1.txt b/file1.txt
index 123..456 100644
--- a/file1.txt
+++ b/file1.txt
@@ -1,3 +1,4 @@
 Line 1
-Line 2
+Line 2 modified
+Line 3 added
 Line 4
diff --git a/file2.txt b/file2.txt
new file mode 100644
index 0000000..789
--- /dev/null
+++ b/file2.txt
@@ -0,0 +1,2 @@
+New file line 1
+New file line 2
"#;
        std::fs::write(&patch_path, patch_content).unwrap();

        let (_git_temp, extractor) = create_test_extractor();
        let stats = extractor.parse_patch_stats(&patch_path).unwrap();

        assert_eq!(stats.files_changed, 2);
        assert_eq!(stats.lines_added, 4); // 1 modified + 1 added + 2 new file
        assert_eq!(stats.lines_removed, 1);
    }

    #[test]
    fn test_empty_patch_stats() {
        let temp_dir = ::tempfile::TempDir::new().unwrap();
        let patch_path = temp_dir.path().join("empty.patch");

        // Create an empty patch file
        std::fs::write(&patch_path, "").unwrap();

        let (_git_temp, extractor) = create_test_extractor();
        let stats = extractor.parse_patch_stats(&patch_path).unwrap();

        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.lines_added, 0);
        assert_eq!(stats.lines_removed, 0);
    }

    #[test]
    fn test_binary_file_patch_stats() {
        let temp_dir = ::tempfile::TempDir::new().unwrap();
        let patch_path = temp_dir.path().join("binary.patch");

        // Create a patch with binary file changes
        let patch_content = r#"From abc123 Mon Sep 17 00:00:00 2001
From: Test User <test@example.com>
Date: Mon, 17 Sep 2023 12:00:00 +0000
Subject: [PATCH] Add binary file

diff --git a/image.png b/image.png
new file mode 100644
index 0000000..123456
Binary files /dev/null and b/image.png differ
"#;
        std::fs::write(&patch_path, patch_content).unwrap();

        let (_git_temp, extractor) = create_test_extractor();
        let stats = extractor.parse_patch_stats(&patch_path).unwrap();

        assert_eq!(stats.files_changed, 1);
        // Binary files don't contribute to line counts
        assert_eq!(stats.lines_added, 0);
        assert_eq!(stats.lines_removed, 0);
    }

    #[test]
    fn test_progress_callback() {
        use std::sync::{Arc, Mutex};

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        let callback: ProgressCallback = Box::new(move |msg| {
            messages_clone.lock().unwrap().push(msg.to_string());
        });

        let options = ExtractionOptions {
            session_name: "test-session".to_string(),
            commit_message: "Test commit".to_string(),
            target_branch: None,
            include_binary: true,
            preserve_permissions: true,
            progress_callback: Some(callback),
        };

        let (_git_temp, extractor) = create_test_extractor();
        extractor.report_progress(&options, "Test message");

        let recorded = messages.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], "Test message");
    }
}
