use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProfile {
    PermissiveOpen,
}

impl SandboxProfile {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "permissive-open" => Some(Self::PermissiveOpen),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::PermissiveOpen => "permissive-open",
        }
    }

    pub fn content(&self) -> &'static str {
        match self {
            Self::PermissiveOpen => include_str!("profiles/permissive-open.sb"),
        }
    }
}

pub fn extract_profile(profile_name: &str) -> Result<PathBuf> {
    let profile = SandboxProfile::from_name(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown sandbox profile: {}", profile_name))?;

    // Create a temporary directory for the profile
    let temp_dir = std::env::temp_dir().join("para-sandbox-profiles");
    fs::create_dir_all(&temp_dir).context("Failed to create sandbox profiles directory")?;

    let profile_path = temp_dir.join(format!("{}.sb", profile.name()));

    // Write the profile content to the file
    fs::write(&profile_path, profile.content()).with_context(|| {
        format!(
            "Failed to write sandbox profile to {}",
            profile_path.display()
        )
    })?;

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
        assert_eq!(SandboxProfile::from_name("unknown"), None);
    }

    #[test]
    fn test_profile_name() {
        assert_eq!(SandboxProfile::PermissiveOpen.name(), "permissive-open");
    }

    #[test]
    fn test_profile_content() {
        let content = SandboxProfile::PermissiveOpen.content();
        assert!(content.contains("(version 1)"));
        assert!(content.contains("(allow default)"));
        assert!(content.contains("(deny file-write*)"));
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
            .contains("Unknown sandbox profile"));
    }
}
