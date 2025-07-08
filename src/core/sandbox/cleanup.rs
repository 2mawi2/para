use std::fs;

/// Cleanup old sandbox profile files
pub fn cleanup_old_profiles() -> anyhow::Result<()> {
    let temp_base = std::env::temp_dir();

    // Look for all para-sandbox-* directories (both old and new naming patterns)
    let entries = fs::read_dir(&temp_base)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Handle both old format (para-sandbox-profiles-*) and new format (para-sandbox-*)
            if (name.starts_with("para-sandbox-profiles-") || name.starts_with("para-sandbox-"))
                && path.is_dir()
            {
                cleanup_profile_directory(&path)?;
            }
        }
    }

    Ok(())
}

fn cleanup_profile_directory(temp_dir: &std::path::Path) -> anyhow::Result<()> {
    if !temp_dir.exists() {
        return Ok(());
    }

    // Get all .sb files in the directory
    let entries = fs::read_dir(temp_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("sb") {
            // Check if file is older than 1 hour
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        // Remove files older than 1 hour
                        if elapsed.as_secs() > 3600 {
                            if let Err(e) = fs::remove_file(&path) {
                                eprintln!(
                                    "Failed to clean up old sandbox profile {}: {}",
                                    path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Cleanup hook to be called on startup
#[allow(dead_code)]
pub fn cleanup_on_startup() {
    if let Err(e) = cleanup_old_profiles() {
        eprintln!("Failed to clean up old sandbox profiles: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    // Helper function to create a test file with specific age
    fn create_test_file_with_age(path: &Path, content: &str, age_hours: u64) -> anyhow::Result<()> {
        fs::write(path, content)?;

        // Set file modification time to simulate age
        if age_hours > 0 {
            let older_time = SystemTime::now() - Duration::from_secs(age_hours * 3600);
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let metadata = fs::metadata(path)?;
                let atime = metadata.atime();
                let mtime = older_time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;

                // Use libc to set file times
                unsafe {
                    let path_cstr = std::ffi::CString::new(path.to_string_lossy().to_string())?;
                    let times = [
                        libc::timespec {
                            tv_sec: atime,
                            tv_nsec: 0,
                        },
                        libc::timespec {
                            tv_sec: mtime,
                            tv_nsec: 0,
                        },
                    ];
                    libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0);
                }
            }
            #[cfg(not(unix))]
            {
                // For non-Unix systems, we'll use filetime crate approach
                let ft = filetime::FileTime::from_system_time(older_time);
                filetime::set_file_mtime(path, ft)?;
            }
        }
        Ok(())
    }

    // Helper function to create test directory with sandbox files
    fn setup_test_sandbox_dir(base_dir: &Path, name: &str) -> anyhow::Result<std::path::PathBuf> {
        let sandbox_dir = base_dir.join(name);
        fs::create_dir_all(&sandbox_dir)?;
        Ok(sandbox_dir)
    }

    #[test]
    fn test_cleanup_nonexistent_dir() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("nonexistent");

        // Should not error when directory doesn't exist
        let result = cleanup_profile_directory(&nonexistent);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_removes_old_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create old .sb file (2 hours old)
        let old_file = sandbox_dir.join("old.sb");
        create_test_file_with_age(&old_file, "old content", 2)?;

        // Verify file exists before cleanup
        assert!(old_file.exists());

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Verify old file was removed
        assert!(!old_file.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_preserves_new_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create new .sb file (current time)
        let new_file = sandbox_dir.join("new.sb");
        create_test_file_with_age(&new_file, "new content", 0)?;

        // Verify file exists before cleanup
        assert!(new_file.exists());

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Verify new file was preserved
        assert!(new_file.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_handles_exactly_one_hour_old_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create file that's exactly 1 hour old
        let exactly_hour_file = sandbox_dir.join("exactly_hour.sb");
        create_test_file_with_age(&exactly_hour_file, "exactly hour content", 1)?;

        // Create file that's slightly more than 1 hour old
        let slightly_older_file = sandbox_dir.join("slightly_older.sb");
        fs::write(&slightly_older_file, "slightly older content")?;

        // Manually set time to be 1 hour and 1 second old
        let older_time = SystemTime::now() - Duration::from_secs(3601);
        #[cfg(not(unix))]
        {
            let ft = filetime::FileTime::from_system_time(older_time);
            filetime::set_file_mtime(&slightly_older_file, ft)?;
        }
        #[cfg(unix)]
        {
            unsafe {
                let path_cstr =
                    std::ffi::CString::new(slightly_older_file.to_string_lossy().to_string())?;
                let mtime = older_time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;
                let times = [
                    libc::timespec {
                        tv_sec: mtime,
                        tv_nsec: 0,
                    },
                    libc::timespec {
                        tv_sec: mtime,
                        tv_nsec: 0,
                    },
                ];
                libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0);
            }
        }

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // File exactly 1 hour old should be preserved (not > 1 hour)
        assert!(exactly_hour_file.exists());

        // File slightly older than 1 hour should be removed
        assert!(!slightly_older_file.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_only_processes_sb_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create old .sb file (should be removed)
        let old_sb_file = sandbox_dir.join("old.sb");
        create_test_file_with_age(&old_sb_file, "old sb content", 2)?;

        // Create old files with different extensions (should be preserved)
        let old_txt_file = sandbox_dir.join("old.txt");
        create_test_file_with_age(&old_txt_file, "old txt content", 2)?;

        let old_log_file = sandbox_dir.join("old.log");
        create_test_file_with_age(&old_log_file, "old log content", 2)?;

        let old_no_ext_file = sandbox_dir.join("old_no_ext");
        create_test_file_with_age(&old_no_ext_file, "old no ext content", 2)?;

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Only .sb file should be removed
        assert!(!old_sb_file.exists());
        assert!(old_txt_file.exists());
        assert!(old_log_file.exists());
        assert!(old_no_ext_file.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_ignores_non_sb_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create files with similar names but different extensions
        let sb_file = sandbox_dir.join("test.sb");
        let sbx_file = sandbox_dir.join("test.sbx");
        let sb_txt_file = sandbox_dir.join("test.sb.txt");
        let txt_sb_file = sandbox_dir.join("test.txt.sb");

        // Make all files old (2 hours)
        create_test_file_with_age(&sb_file, "sb content", 2)?;
        create_test_file_with_age(&sbx_file, "sbx content", 2)?;
        create_test_file_with_age(&sb_txt_file, "sb.txt content", 2)?;
        create_test_file_with_age(&txt_sb_file, "txt.sb content", 2)?;

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Only .sb files should be removed
        assert!(!sb_file.exists());
        assert!(sbx_file.exists());
        assert!(sb_txt_file.exists());
        assert!(!txt_sb_file.exists()); // This ends with .sb so should be removed

        Ok(())
    }

    #[test]
    fn test_cleanup_empty_directory() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let empty_dir = setup_test_sandbox_dir(temp.path(), "empty-sandbox")?;

        // Directory exists but is empty
        assert!(empty_dir.exists());

        // Should not error on empty directory
        let result = cleanup_profile_directory(&empty_dir);
        assert!(result.is_ok());

        // Directory should still exist
        assert!(empty_dir.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_mixed_file_ages() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "test-sandbox")?;

        // Create files with different ages
        let very_old_file = sandbox_dir.join("very_old.sb");
        create_test_file_with_age(&very_old_file, "very old", 10)?;

        let old_file = sandbox_dir.join("old.sb");
        create_test_file_with_age(&old_file, "old", 2)?;

        let recent_file = sandbox_dir.join("recent.sb");
        create_test_file_with_age(&recent_file, "recent", 0)?;

        let half_hour_file = sandbox_dir.join("half_hour.sb");
        let half_hour_ago = SystemTime::now() - Duration::from_secs(1800); // 30 minutes
        fs::write(&half_hour_file, "half hour content")?;
        #[cfg(not(unix))]
        {
            let ft = filetime::FileTime::from_system_time(half_hour_ago);
            filetime::set_file_mtime(&half_hour_file, ft)?;
        }
        #[cfg(unix)]
        {
            unsafe {
                let path_cstr =
                    std::ffi::CString::new(half_hour_file.to_string_lossy().to_string())?;
                let mtime = half_hour_ago
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs() as i64;
                let times = [
                    libc::timespec {
                        tv_sec: mtime,
                        tv_nsec: 0,
                    },
                    libc::timespec {
                        tv_sec: mtime,
                        tv_nsec: 0,
                    },
                ];
                libc::utimensat(libc::AT_FDCWD, path_cstr.as_ptr(), times.as_ptr(), 0);
            }
        }

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Only files older than 1 hour should be removed
        assert!(!very_old_file.exists());
        assert!(!old_file.exists());
        assert!(recent_file.exists());
        assert!(half_hour_file.exists());

        Ok(())
    }

    #[test]
    fn test_cleanup_with_real_file_system() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "real-fs-test")?;

        // Create a variety of files
        let files_to_create = [
            ("keep_new.sb", 0),
            ("remove_old.sb", 3),
            ("keep_txt.txt", 5),
            ("keep_no_ext", 2),
            ("remove_very_old.sb", 24),
        ];

        let mut expected_to_exist = Vec::new();
        let mut expected_to_remove = Vec::new();

        for (filename, age_hours) in files_to_create {
            let file_path = sandbox_dir.join(filename);
            create_test_file_with_age(&file_path, &format!("content {filename}"), age_hours)?;

            // Predict what should happen
            if filename.ends_with(".sb") && age_hours > 1 {
                expected_to_remove.push(file_path);
            } else {
                expected_to_exist.push(file_path);
            }
        }

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Verify predictions
        for file_path in expected_to_exist {
            assert!(
                file_path.exists(),
                "File should exist: {}",
                file_path.display()
            );
        }
        for file_path in expected_to_remove {
            assert!(
                !file_path.exists(),
                "File should be removed: {}",
                file_path.display()
            );
        }

        Ok(())
    }

    #[test]
    fn test_cleanup_verifies_actual_removal() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let sandbox_dir = setup_test_sandbox_dir(temp.path(), "verify-removal")?;

        // Create multiple old .sb files
        let old_files = ["old1.sb", "old2.sb", "old3.sb"];
        let mut old_file_paths = Vec::new();

        for filename in old_files {
            let file_path = sandbox_dir.join(filename);
            create_test_file_with_age(&file_path, &format!("content {filename}"), 2)?;
            old_file_paths.push(file_path);
        }

        // Verify files exist before cleanup
        for file_path in &old_file_paths {
            assert!(
                file_path.exists(),
                "File should exist before cleanup: {}",
                file_path.display()
            );
        }

        // Run cleanup
        cleanup_profile_directory(&sandbox_dir)?;

        // Verify all old files were actually removed
        for file_path in &old_file_paths {
            assert!(
                !file_path.exists(),
                "File should be removed after cleanup: {}",
                file_path.display()
            );
        }

        Ok(())
    }

    #[test]
    fn test_cleanup_with_old_files_original() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let profiles_dir = temp.path().join("para-sandbox-profiles");
        fs::create_dir_all(&profiles_dir)?;

        // Create a test file
        let test_file = profiles_dir.join("test.sb");
        fs::write(&test_file, "test content")?;

        // File is new, so cleanup shouldn't remove it
        let result = cleanup_profile_directory(&profiles_dir);
        assert!(result.is_ok());
        assert!(test_file.exists());

        Ok(())
    }
}
