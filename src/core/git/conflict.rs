use crate::core::git::repository::{execute_git_command, GitRepository};
use crate::utils::error::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub file_path: PathBuf,
    pub conflict_type: ConflictType,
    pub markers: Vec<ConflictMarker>,
}

#[derive(Debug, Clone)]
pub enum ConflictType {
    Content,
    AddAdd,
    DeleteModify,
    ModifyDelete,
    Rename,
}

#[derive(Debug, Clone)]
pub struct ConflictMarker {
    pub start_line: usize,
    pub end_line: usize,
}

pub struct ConflictManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> ConflictManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self { repo }
    }

    pub fn detect_conflicts(&self) -> Result<Vec<ConflictInfo>> {
        let conflicted_files = self.get_conflicted_file_paths()?;
        let mut conflicts = Vec::new();

        for file_path in conflicted_files {
            if let Ok(conflict_info) = self.analyze_conflict(&file_path) {
                conflicts.push(conflict_info);
            }
        }

        Ok(conflicts)
    }

    pub fn get_conflicted_file_paths(&self) -> Result<Vec<PathBuf>> {
        let output = execute_git_command(self.repo, &["diff", "--name-only", "--diff-filter=U"])?;

        Ok(output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| PathBuf::from(line.trim()))
            .collect())
    }

    pub fn analyze_conflict(&self, file_path: &Path) -> Result<ConflictInfo> {
        let full_path = self.repo.work_dir.join(file_path);
        let content = fs::read_to_string(&full_path).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to read conflicted file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        let conflict_type = self.determine_conflict_type(file_path)?;
        let markers = self.parse_conflict_markers(&content)?;

        Ok(ConflictInfo {
            file_path: file_path.to_path_buf(),
            conflict_type,
            markers,
        })
    }

    fn determine_conflict_type(&self, file_path: &Path) -> Result<ConflictType> {
        let status_output = execute_git_command(
            self.repo,
            &["status", "--porcelain", &file_path.to_string_lossy()],
        )?;

        let status_code = status_output.chars().nth(1).unwrap_or(' ');

        match status_code {
            'U' => Ok(ConflictType::Content),
            'A' => Ok(ConflictType::AddAdd),
            'D' => Ok(ConflictType::DeleteModify),
            'M' => Ok(ConflictType::ModifyDelete),
            'R' => Ok(ConflictType::Rename),
            _ => Ok(ConflictType::Content),
        }
    }

    fn parse_conflict_markers(&self, content: &str) -> Result<Vec<ConflictMarker>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut markers = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("<<<<<<<") {
                if let Some(marker) = self.parse_single_conflict_marker(&lines, i)? {
                    i = marker.end_line + 1;
                    markers.push(marker);
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        Ok(markers)
    }

    fn parse_single_conflict_marker(
        &self,
        lines: &[&str],
        start: usize,
    ) -> Result<Option<ConflictMarker>> {
        let mut middle_line = None;
        let mut end_line = None;

        #[allow(clippy::needless_range_loop)]
        for i in (start + 1)..lines.len() {
            if lines[i].starts_with("=======") && middle_line.is_none() {
                middle_line = Some(i);
            } else if lines[i].starts_with(">>>>>>>") {
                end_line = Some(i);
                break;
            }
        }

        if let Some(end) = end_line {
            Ok(Some(ConflictMarker {
                start_line: start,
                end_line: end,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_conflict_summary(&self) -> Result<String> {
        let conflicts = self.detect_conflicts()?;

        if conflicts.is_empty() {
            return Ok("No conflicts detected".to_string());
        }

        let mut summary = format!("Found {} conflicted files:\n\n", conflicts.len());

        for conflict in &conflicts {
            summary.push_str(&format!(
                "📁 {} ({:?})\n",
                conflict.file_path.display(),
                conflict.conflict_type
            ));

            for (i, marker) in conflict.markers.iter().enumerate() {
                summary.push_str(&format!(
                    "  Conflict #{}: lines {}-{}\n",
                    i + 1,
                    marker.start_line + 1,
                    marker.end_line + 1
                ));
            }
            summary.push('\n');
        }

        summary.push_str(&self.get_resolution_instructions());
        Ok(summary)
    }

    pub fn get_resolution_instructions(&self) -> String {
        "Resolution Steps:\n\
        1. Edit each conflicted file to resolve conflicts\n\
        2. Remove conflict markers (<<<<<<< ======= >>>>>>>)\n\
        3. Stage resolved files: git add <file>\n\
        4. Continue integration: para continue\n\
        \n\
        Abort integration: para integrate --abort"
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepository) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        let repo = GitRepository::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, repo)
    }

    #[test]
    fn test_parse_conflict_markers() {
        let (_temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let content = "line1\n\
            <<<<<<< HEAD\n\
            our content\n\
            =======\n\
            their content\n\
            >>>>>>> branch\n\
            line2";

        let markers = conflict_manager
            .parse_conflict_markers(content)
            .expect("Failed to parse markers");

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].start_line, 1);
        assert_eq!(markers[0].end_line, 5);
    }

    #[test]
    fn test_conflict_info_creation() {
        let (temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let test_file = temp_dir.path().join("conflict.txt");
        let content = "line1\n\
            <<<<<<< HEAD\n\
            our version\n\
            =======\n\
            their version\n\
            >>>>>>> feature\n\
            line2";

        fs::write(&test_file, content).expect("Failed to write test file");

        let conflict = conflict_manager
            .analyze_conflict(&PathBuf::from("conflict.txt"))
            .expect("Failed to analyze conflict");

        assert_eq!(conflict.file_path, PathBuf::from("conflict.txt"));
        assert_eq!(conflict.markers.len(), 1);
    }

    #[test]
    fn test_multiple_conflicts_in_file() {
        let (_temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let content = "line1\n\
            <<<<<<< HEAD\n\
            first conflict ours\n\
            =======\n\
            first conflict theirs\n\
            >>>>>>> branch\n\
            middle line\n\
            <<<<<<< HEAD\n\
            second conflict ours\n\
            =======\n\
            second conflict theirs\n\
            >>>>>>> branch\n\
            final line";

        let markers = conflict_manager
            .parse_conflict_markers(content)
            .expect("Failed to parse markers");

        assert_eq!(markers.len(), 2);
    }

    #[test]
    fn test_conflict_resolution_instructions() {
        let (_temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let instructions = conflict_manager.get_resolution_instructions();

        assert!(instructions.contains("Edit each conflicted file"));
        assert!(instructions.contains("Remove conflict markers"));
        assert!(instructions.contains("git add"));
        assert!(instructions.contains("para continue"));
    }
}
