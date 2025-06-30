use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProfile {
    PermissiveOpen,
    PermissiveClosed,
    RestrictiveClosed,
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
            eprintln!("⚠️  Invalid profile name format: {}", s);
            return None;
        }

        match s {
            "permissive-open" => Some(Self::PermissiveOpen),
            "permissive-closed" => Some(Self::PermissiveClosed),
            "restrictive-closed" => Some(Self::RestrictiveClosed),
            _ => {
                eprintln!("⚠️  Unknown sandbox profile: {}", s);
                None
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::PermissiveOpen => "permissive-open",
            Self::PermissiveClosed => "permissive-closed",
            Self::RestrictiveClosed => "restrictive-closed",
        }
    }

    pub fn content(&self) -> &'static str {
        match self {
            Self::PermissiveOpen => include_str!("profiles/permissive-open.sb"),
            Self::PermissiveClosed => include_str!("profiles/permissive-closed.sb"),
            Self::RestrictiveClosed => include_str!("profiles/restrictive-closed.sb"),
        }
    }
}

pub fn extract_profile(profile_name: &str) -> Result<PathBuf> {
    // Validate profile name first (this includes format validation)
    let profile = SandboxProfile::from_name(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Invalid or unknown sandbox profile: {}", profile_name))?;

    // Create a temporary directory for the profile with a unique suffix to avoid conflicts
    let temp_dir = std::env::temp_dir()
        .join(format!("para-sandbox-profiles-{}", std::process::id()));
    
    // Try to create directory, if it exists with wrong permissions try to fix them
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        // If directory exists, try to set permissions and retry
        if temp_dir.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mut perms) = fs::metadata(&temp_dir).map(|m| m.permissions()) {
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&temp_dir, perms);
                }
            }
            // Try one more time
            fs::create_dir_all(&temp_dir)
                .context("Failed to create sandbox profiles directory after permission fix")?;
        } else {
            return Err(e).context("Failed to create sandbox profiles directory");
        }
    }

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
            SandboxProfile::from_name("permissive-open"),
            Some(SandboxProfile::PermissiveOpen)
        );
        assert_eq!(
            SandboxProfile::from_name("permissive-closed"),
            Some(SandboxProfile::PermissiveClosed)
        );
        assert_eq!(
            SandboxProfile::from_name("restrictive-closed"),
            Some(SandboxProfile::RestrictiveClosed)
        );
        assert_eq!(SandboxProfile::from_name("unknown"), None);
    }

    #[test]
    fn test_profile_name() {
        assert_eq!(SandboxProfile::PermissiveOpen.name(), "permissive-open");
        assert_eq!(SandboxProfile::PermissiveClosed.name(), "permissive-closed");
        assert_eq!(
            SandboxProfile::RestrictiveClosed.name(),
            "restrictive-closed"
        );
    }

    #[test]
    fn test_profile_content() {
        let content = SandboxProfile::PermissiveOpen.content();
        assert!(content.contains("(version 1)"));
        assert!(content.contains("(allow default)"));
        assert!(content.contains("(deny file-write*)"));

        let content_closed = SandboxProfile::PermissiveClosed.content();
        assert!(content_closed.contains("(deny network*)"));
        assert!(content_closed.contains("(allow network*"));
        assert!(content_closed.contains("localhost"));

        let content_restrictive = SandboxProfile::RestrictiveClosed.content();
        assert!(content_restrictive.contains("(deny default)"));
        assert!(content_restrictive.contains("(allow file-read*)"));
        assert!(content_restrictive.contains("(allow process-exec)"));
    }

    #[test]
    fn test_extract_profile() -> Result<()> {
        let profile_path = extract_profile("permissive-open")?;
        assert!(profile_path.exists());
        assert!(profile_path.extension().unwrap() == "sb");

        let content = fs::read_to_string(&profile_path)?;
        assert!(content.contains("(version 1)"));

        Ok(())
    }

    #[test]
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
        assert!(SandboxProfile::from_name("permissive-open").is_some());
        assert!(SandboxProfile::from_name("permissive-closed").is_some());
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
            assert!(result.is_err(), "Should reject invalid name: {}", name);
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_extracted_profile_permissions() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let profile_path = extract_profile("permissive-open")?;
        let metadata = fs::metadata(&profile_path)?;
        let perms = metadata.permissions();

        // Check that file is read-only (0o444)
        assert_eq!(perms.mode() & 0o777, 0o444);

        Ok(())
    }
}
