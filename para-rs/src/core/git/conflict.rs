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
    pub middle_line: usize,
    pub end_line: usize,
    pub ours_content: String,
    pub theirs_content: String,
}

#[derive(Debug)]
pub struct ConflictResolution {
    pub resolved_files: Vec<PathBuf>,
    pub remaining_conflicts: Vec<ConflictInfo>,
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
        let content = fs::read_to_string(&full_path)
            .map_err(|e| ParaError::file_operation(format!(
                "Failed to read conflicted file {}: {}", 
                file_path.display(), 
                e
            )))?;

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

        for i in (start + 1)..lines.len() {
            if lines[i].starts_with("=======") && middle_line.is_none() {
                middle_line = Some(i);
            } else if lines[i].starts_with(">>>>>>>") {
                end_line = Some(i);
                break;
            }
        }

        if let (Some(middle), Some(end)) = (middle_line, end_line) {
            let ours_content = lines[(start + 1)..middle].join("\n");
            let theirs_content = lines[(middle + 1)..end].join("\n");

            Ok(Some(ConflictMarker {
                start_line: start,
                middle_line: middle,
                end_line: end,
                ours_content,
                theirs_content,
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
        Abort integration: para integrate --abort".to_string()
    }

    pub fn validate_resolution(&self) -> Result<ConflictResolution> {
        let all_conflicts = self.detect_conflicts()?;
        let mut resolved_files = Vec::new();
        let mut remaining_conflicts = Vec::new();

        for conflict in all_conflicts {
            let full_path = self.repo.work_dir.join(&conflict.file_path);
            
            if let Ok(content) = fs::read_to_string(&full_path) {
                if self.has_conflict_markers(&content) {
                    remaining_conflicts.push(conflict);
                } else {
                    resolved_files.push(conflict.file_path);
                }
            } else {
                remaining_conflicts.push(conflict);
            }
        }

        Ok(ConflictResolution {
            resolved_files,
            remaining_conflicts,
        })
    }

    fn has_conflict_markers(&self, content: &str) -> bool {
        content.lines().any(|line| {
            line.starts_with("<<<<<<<") || 
            line.starts_with("=======") || 
            line.starts_with(">>>>>>>")
        })
    }

    pub fn stage_resolved_files(&self) -> Result<Vec<PathBuf>> {
        let resolution = self.validate_resolution()?;
        
        if !resolution.remaining_conflicts.is_empty() {
            return Err(ParaError::git_operation(format!(
                "Cannot stage files: {} conflicts remain unresolved",
                resolution.remaining_conflicts.len()
            )));
        }

        for file in &resolution.resolved_files {
            execute_git_command(
                self.repo,
                &["add", &file.to_string_lossy()],
            )?;
        }

        Ok(resolution.resolved_files)
    }

    pub fn show_conflict_diff(&self, file_path: &Path) -> Result<String> {
        let conflict = self.analyze_conflict(file_path)?;
        let mut diff = format!("Conflict in {}:\n\n", file_path.display());

        for (i, marker) in conflict.markers.iter().enumerate() {
            diff.push_str(&format!("Conflict #{} (lines {}-{}):\n", 
                i + 1, 
                marker.start_line + 1, 
                marker.end_line + 1
            ));
            
            diff.push_str("<<<<<<< HEAD (Current changes)\n");
            diff.push_str(&marker.ours_content);
            diff.push_str("\n=======\n");
            diff.push_str(&marker.theirs_content);
            diff.push_str("\n>>>>>>> (Incoming changes)\n\n");
        }

        Ok(diff)
    }

    pub fn suggest_resolution_strategy(&self, conflict: &ConflictInfo) -> String {
        match conflict.conflict_type {
            ConflictType::Content => {
                "Content conflict: Manually edit the file to combine changes or choose one version"
            }
            ConflictType::AddAdd => {
                "Both branches added this file: Choose which version to keep or merge contents"
            }
            ConflictType::DeleteModify => {
                "File deleted in one branch, modified in another: Choose to keep modified version or delete"
            }
            ConflictType::ModifyDelete => {
                "File modified in one branch, deleted in another: Choose to keep modifications or delete"
            }
            ConflictType::Rename => {
                "File renamed differently in both branches: Choose one name or create new name"
            }
        }.to_string()
    }

    pub fn get_detailed_conflict_info(&self, file_path: &Path) -> Result<String> {
        let conflict = self.analyze_conflict(file_path)?;
        let mut info = format!("Detailed conflict information for {}:\n\n", file_path.display());

        info.push_str(&format!("Conflict type: {:?}\n", conflict.conflict_type));
        info.push_str(&format!("Number of conflicts: {}\n\n", conflict.markers.len()));

        info.push_str("Suggested resolution strategy:\n");
        info.push_str(&self.suggest_resolution_strategy(&conflict));
        info.push_str("\n\n");

        info.push_str(&self.show_conflict_diff(file_path)?);

        Ok(info)
    }

    pub fn auto_resolve_simple_conflicts(&self) -> Result<Vec<PathBuf>> {
        let conflicts = self.detect_conflicts()?;
        let mut auto_resolved = Vec::new();

        for conflict in conflicts {
            if self.can_auto_resolve(&conflict) {
                if let Ok(()) = self.perform_auto_resolution(&conflict) {
                    auto_resolved.push(conflict.file_path);
                }
            }
        }

        Ok(auto_resolved)
    }

    fn can_auto_resolve(&self, conflict: &ConflictInfo) -> bool {
        match conflict.conflict_type {
            ConflictType::Content => {
                conflict.markers.iter().all(|marker| {
                    marker.ours_content.trim().is_empty() || 
                    marker.theirs_content.trim().is_empty()
                })
            }
            _ => false,
        }
    }

    fn perform_auto_resolution(&self, conflict: &ConflictInfo) -> Result<()> {
        let full_path = self.repo.work_dir.join(&conflict.file_path);
        let content = fs::read_to_string(&full_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut resolved_lines = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("<<<<<<<") {
                if let Some(marker) = self.find_marker_at_line(&conflict.markers, i) {
                    let resolution = if marker.ours_content.trim().is_empty() {
                        &marker.theirs_content
                    } else {
                        &marker.ours_content
                    };
                    
                    resolved_lines.push(resolution.as_str());
                    i = marker.end_line + 1;
                } else {
                    resolved_lines.push(lines[i]);
                    i += 1;
                }
            } else {
                resolved_lines.push(lines[i]);
                i += 1;
            }
        }

        let resolved_content = resolved_lines.join("\n");
        fs::write(&full_path, resolved_content)?;

        Ok(())
    }

    fn find_marker_at_line<'b>(&self, markers: &'b [ConflictMarker], line: usize) -> Option<&'b ConflictMarker> {
        markers.iter().find(|marker| marker.start_line == line)
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
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
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
        assert_eq!(markers[0].middle_line, 3);
        assert_eq!(markers[0].end_line, 5);
        assert_eq!(markers[0].ours_content, "our content");
        assert_eq!(markers[0].theirs_content, "their content");
    }

    #[test]
    fn test_has_conflict_markers() {
        let (_temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let content_with_conflicts = "line1\n<<<<<<< HEAD\ncontent\n=======\nother\n>>>>>>> branch";
        let content_without_conflicts = "line1\nline2\nline3";

        assert!(conflict_manager.has_conflict_markers(content_with_conflicts));
        assert!(!conflict_manager.has_conflict_markers(content_without_conflicts));
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
        assert_eq!(conflict.markers[0].ours_content, "our version");
        assert_eq!(conflict.markers[0].theirs_content, "their version");
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
        assert_eq!(markers[0].ours_content, "first conflict ours");
        assert_eq!(markers[0].theirs_content, "first conflict theirs");
        assert_eq!(markers[1].ours_content, "second conflict ours");
        assert_eq!(markers[1].theirs_content, "second conflict theirs");
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

    #[test]
    fn test_can_auto_resolve() {
        let (_temp_dir, repo) = setup_test_repo();
        let conflict_manager = ConflictManager::new(&repo);

        let auto_resolvable = ConflictInfo {
            file_path: PathBuf::from("test.txt"),
            conflict_type: ConflictType::Content,
            markers: vec![ConflictMarker {
                start_line: 0,
                middle_line: 2,
                end_line: 4,
                ours_content: "content".to_string(),
                theirs_content: "".to_string(),
            }],
        };

        let not_auto_resolvable = ConflictInfo {
            file_path: PathBuf::from("test.txt"),
            conflict_type: ConflictType::Content,
            markers: vec![ConflictMarker {
                start_line: 0,
                middle_line: 2,
                end_line: 4,
                ours_content: "our content".to_string(),
                theirs_content: "their content".to_string(),
            }],
        };

        assert!(conflict_manager.can_auto_resolve(&auto_resolvable));
        assert!(!conflict_manager.can_auto_resolve(&not_auto_resolvable));
    }
}