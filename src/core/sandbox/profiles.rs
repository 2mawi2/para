//! Sandbox profiles for macOS sandboxing
//! This module is primarily used on macOS for sandbox-exec functionality
//! Profile validation is performed on all platforms for consistency

#[cfg(target_os = "macos")]
use anyhow::{Context, Result};
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProfile {
    Standard,
}

/// Validate profile name contains only safe characters
fn validate_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 50  // Reasonable length limit
        && name.chars().all(|c| c.is_alphanumeric() || c == '-')
}

impl SandboxProfile {
    pub fn from_name(s: &str) -> Option<Self> {
        // Validate profile name format (alphanumeric and hyphen only)
        if !validate_profile_name(s) {
            eprintln!("⚠️  Invalid profile name format: {s}");
            return None;
        }

        match s {
            "standard" => Some(Self::Standard),
            // Legacy names for backwards compatibility
            "permissive" => Some(Self::Standard),
            "restrictive" => Some(Self::Standard),
            "permissive-open" => Some(Self::Standard),
            "restrictive-closed" => Some(Self::Standard),
            "development" => Some(Self::Standard),
            "isolated" => Some(Self::Standard),
            _ => {
                eprintln!("⚠️  Unknown sandbox profile: {s}");
                None
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
        }
    }

    #[cfg(target_os = "macos")]
    pub fn content(&self) -> &'static str {
        match self {
            Self::Standard => include_str!("profiles/standard.sb"),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn extract_profile(profile_name: &str) -> Result<PathBuf> {
    // Validate profile name first (this includes format validation)
    let profile = SandboxProfile::from_name(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Invalid or unknown sandbox profile: {}", profile_name))?;

    // Create a unique temporary directory for each extraction to avoid conflicts
    use uuid::Uuid;
    let unique_id = Uuid::new_v4();
    let temp_dir = std::env::temp_dir().join(format!("para-sandbox-{unique_id}"));

    // Create the directory fresh each time
    fs::create_dir_all(&temp_dir).context("Failed to create sandbox profiles directory")?;

    // Use the validated profile name to prevent path injection
    let profile_path = temp_dir.join(format!("{}.sb", profile.name()));

    // Validate that the profile content is not empty
    let content = profile.content();
    if content.is_empty() {
        return Err(anyhow::anyhow!("Sandbox profile content is empty"));
    }

    // Write the profile content to the file with proper permissions
    fs::write(&profile_path, content).with_context(|| {
        format!(
            "Failed to write sandbox profile to {}",
            profile_path.display()
        )
    })?;

    // Set read-only permissions on the profile file for security
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&profile_path)?.permissions();
        perms.set_mode(0o444); // Read-only for all
        fs::set_permissions(&profile_path, perms)?;
    }

    Ok(profile_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_from_name() {
        assert_eq!(
            SandboxProfile::from_name("standard"),
            Some(SandboxProfile::Standard)
        );
        // Test legacy names for backwards compatibility
        assert_eq!(
            SandboxProfile::from_name("permissive"),
            Some(SandboxProfile::Standard)
        );
        assert_eq!(
            SandboxProfile::from_name("restrictive"),
            Some(SandboxProfile::Standard)
        );
        assert_eq!(
            SandboxProfile::from_name("permissive-open"),
            Some(SandboxProfile::Standard)
        );
        assert_eq!(
            SandboxProfile::from_name("restrictive-closed"),
            Some(SandboxProfile::Standard)
        );
        assert_eq!(SandboxProfile::from_name("unknown"), None);
    }

    #[test]
    fn test_profile_name() {
        assert_eq!(SandboxProfile::Standard.name(), "standard");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_profile_content() {
        let content = SandboxProfile::Standard.content();
        assert!(content.contains("(version 1)"));
        assert!(content.contains("(deny default)"));
        assert!(content.contains("(allow file-read*)"));
        assert!(content.contains("(allow network*)"));
        assert!(content.contains("Para Sandboxing Profile - Standard"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_extract_profile() -> Result<()> {
        let profile_path = extract_profile("standard")?;
        assert!(profile_path.exists());
        assert!(profile_path.extension().unwrap() == "sb");

        let content = std::fs::read_to_string(&profile_path)?;
        assert!(content.contains("(version 1)"));

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_extract_unknown_profile() {
        let result = extract_profile("unknown-profile");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid or unknown sandbox profile"));
    }

    #[test]
    fn test_profile_name_validation() {
        // Valid profile names that exist
        assert!(SandboxProfile::from_name("permissive").is_some());
        assert!(SandboxProfile::from_name("restrictive").is_some());
        // Legacy names
        assert!(SandboxProfile::from_name("permissive-open").is_some());
        assert!(SandboxProfile::from_name("restrictive-closed").is_some());

        // Valid format but unknown profiles
        assert!(SandboxProfile::from_name("test-123").is_none());
        assert!(SandboxProfile::from_name("abc123").is_none());

        // Invalid names (format validation)
        assert!(SandboxProfile::from_name("").is_none()); // Empty
        assert!(SandboxProfile::from_name("test/path").is_none()); // Path injection
        assert!(SandboxProfile::from_name("test..profile").is_none()); // Directory traversal
        assert!(SandboxProfile::from_name("test profile").is_none()); // Space
        assert!(SandboxProfile::from_name("test;rm").is_none()); // Command injection
        assert!(SandboxProfile::from_name(&"a".repeat(51)).is_none()); // Too long
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_profile_extraction_with_invalid_names() {
        // Test various invalid profile names
        let invalid_names = vec![
            "../../../etc/passwd",
            "test/../../secret",
            "profile;rm -rf /",
            "profile$(whoami)",
            "profile`id`",
        ];

        for name in invalid_names {
            let result = extract_profile(name);
            assert!(result.is_err(), "Should reject invalid name: {name}");
        }
    }

    #[test]
    #[cfg(all(unix, target_os = "macos"))]
    fn test_extracted_profile_permissions() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let profile_path = extract_profile("standard")?;
        let metadata = std::fs::metadata(&profile_path)?;
        let perms = metadata.permissions();

        // Check that file is read-only (0o444)
        assert_eq!(perms.mode() & 0o777, 0o444);

        Ok(())
    }
}
