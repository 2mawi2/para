/// Cleanup old sandbox profile files
#[cfg(test)]
pub fn cleanup_old_profiles() -> anyhow::Result<()> {
    use std::fs;
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

#[cfg(test)]
fn cleanup_profile_directory(temp_dir: &std::path::Path) -> anyhow::Result<()> {
    use std::fs;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_nonexistent_dir() {
        // Should not error when directory doesn't exist
        let result = cleanup_old_profiles();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_with_old_files() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let profiles_dir = temp.path().join("para-sandbox-profiles");
        fs::create_dir_all(&profiles_dir)?;

        // Create a test file
        let test_file = profiles_dir.join("test.sb");
        fs::write(&test_file, "test content")?;

        // File is new, so cleanup shouldn't remove it
        let result = cleanup_old_profiles();
        assert!(result.is_ok());
        assert!(test_file.exists());

        Ok(())
    }
}
