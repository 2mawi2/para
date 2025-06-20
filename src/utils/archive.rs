use crate::utils::{ParaError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveBranchInfo {
    pub timestamp: String,
    pub session_name: String,
    pub full_branch_name: String,
}

pub struct ArchiveBranchParser;

impl ArchiveBranchParser {
    pub fn parse_archive_branch(
        branch_name: &str,
        branch_prefix: &str,
    ) -> Result<Option<ArchiveBranchInfo>> {
        let archive_prefix = format!("{}/archived/", branch_prefix);

        if !branch_name.starts_with(&archive_prefix) {
            return Ok(None);
        }

        let suffix = branch_name.strip_prefix(&archive_prefix).unwrap();
        let parts: Vec<&str> = suffix.split('/').collect();

        if parts.len() != 2 {
            return Err(ParaError::InvalidArgs {
                message: format!(
                    "Invalid archived branch format: '{}'. Expected format: '{}/archived/{{timestamp}}/{{session_name}}'",
                    branch_name, branch_prefix
                ),
            });
        }

        let timestamp = parts[0];
        let session_name = parts[1];

        if timestamp.is_empty() {
            return Err(ParaError::InvalidArgs {
                message: format!("Empty timestamp in archived branch: '{}'", branch_name),
            });
        }

        if session_name.is_empty() {
            return Err(ParaError::InvalidArgs {
                message: format!("Empty session name in archived branch: '{}'", branch_name),
            });
        }

        Ok(Some(ArchiveBranchInfo {
            timestamp: timestamp.to_string(),
            session_name: session_name.to_string(),
            full_branch_name: branch_name.to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_archive_branch() {
        let branch_name = "para/archived/20240301-120000/my-session";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix)
            .unwrap()
            .unwrap();

        assert_eq!(result.timestamp, "20240301-120000");
        assert_eq!(result.session_name, "my-session");
        assert_eq!(result.full_branch_name, branch_name);
    }

    #[test]
    fn test_parse_valid_archive_branch_with_different_prefix() {
        let branch_name = "test/archived/20240301-120000/my-session";
        let branch_prefix = "test";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix)
            .unwrap()
            .unwrap();

        assert_eq!(result.timestamp, "20240301-120000");
        assert_eq!(result.session_name, "my-session");
        assert_eq!(result.full_branch_name, branch_name);
    }

    #[test]
    fn test_parse_session_name_with_hyphens() {
        let branch_name = "para/archived/20240301-120000/my-complex-session-name";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix)
            .unwrap()
            .unwrap();

        assert_eq!(result.timestamp, "20240301-120000");
        assert_eq!(result.session_name, "my-complex-session-name");
        assert_eq!(result.full_branch_name, branch_name);
    }

    #[test]
    fn test_parse_session_name_with_underscores() {
        let branch_name = "para/archived/20240301-120000/my_session_name";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix)
            .unwrap()
            .unwrap();

        assert_eq!(result.timestamp, "20240301-120000");
        assert_eq!(result.session_name, "my_session_name");
        assert_eq!(result.full_branch_name, branch_name);
    }

    #[test]
    fn test_parse_non_archive_branch_returns_none() {
        let branch_name = "para/feature/my-feature";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_regular_branch_returns_none() {
        let branch_name = "main";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_archive_branch_wrong_prefix_returns_none() {
        let branch_name = "other/archived/20240301-120000/my-session";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_malformed_archive_branch_missing_session() {
        let branch_name = "para/archived/20240301-120000";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid archived branch format"));
    }

    #[test]
    fn test_parse_malformed_archive_branch_too_many_parts() {
        let branch_name = "para/archived/20240301-120000/my-session/extra";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid archived branch format"));
    }

    #[test]
    fn test_parse_malformed_archive_branch_empty_timestamp() {
        let branch_name = "para/archived//my-session";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty timestamp"));
    }

    #[test]
    fn test_parse_malformed_archive_branch_empty_session() {
        let branch_name = "para/archived/20240301-120000/";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty session name"));
    }

    #[test]
    fn test_parse_archive_branch_with_slashes_in_timestamp() {
        let branch_name = "para/archived/2024/03/01-120000/my-session";
        let branch_prefix = "para";

        let result = ArchiveBranchParser::parse_archive_branch(branch_name, branch_prefix);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid archived branch format"));
    }

    #[test]
    fn test_different_timestamp_formats() {
        let test_cases = vec![
            "20240301-120000",
            "20240301T120000",
            "2024-03-01-12:00:00",
            "1234567890",
            "custom-timestamp",
        ];

        for timestamp in test_cases {
            let branch_name = format!("para/archived/{}/my-session", timestamp);
            let branch_prefix = "para";

            let result = ArchiveBranchParser::parse_archive_branch(&branch_name, branch_prefix)
                .unwrap()
                .unwrap();

            assert_eq!(result.timestamp, timestamp);
            assert_eq!(result.session_name, "my-session");
            assert_eq!(result.full_branch_name, branch_name);
        }
    }

    #[test]
    fn test_archive_branch_info_equality() {
        let info1 = ArchiveBranchInfo {
            timestamp: "20240301-120000".to_string(),
            session_name: "my-session".to_string(),
            full_branch_name: "para/archived/20240301-120000/my-session".to_string(),
        };

        let info2 = ArchiveBranchInfo {
            timestamp: "20240301-120000".to_string(),
            session_name: "my-session".to_string(),
            full_branch_name: "para/archived/20240301-120000/my-session".to_string(),
        };

        let info3 = ArchiveBranchInfo {
            timestamp: "20240301-120000".to_string(),
            session_name: "different-session".to_string(),
            full_branch_name: "para/archived/20240301-120000/different-session".to_string(),
        };

        assert_eq!(info1, info2);
        assert_ne!(info1, info3);
    }

    #[test]
    fn test_archive_branch_info_debug() {
        let info = ArchiveBranchInfo {
            timestamp: "20240301-120000".to_string(),
            session_name: "my-session".to_string(),
            full_branch_name: "para/archived/20240301-120000/my-session".to_string(),
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("timestamp"));
        assert!(debug_str.contains("session_name"));
        assert!(debug_str.contains("full_branch_name"));
    }

    #[test]
    fn test_archive_branch_info_clone() {
        let info = ArchiveBranchInfo {
            timestamp: "20240301-120000".to_string(),
            session_name: "my-session".to_string(),
            full_branch_name: "para/archived/20240301-120000/my-session".to_string(),
        };

        let cloned = info.clone();
        assert_eq!(info, cloned);
    }
}
