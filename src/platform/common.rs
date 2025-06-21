/// Shared functionality for parsing launch file contents
/// This logic is used by both production code and tests
pub fn parse_launch_file_contents(contents: &str, default_ide: &str) -> String {
    if contents.contains("LAUNCH_METHOD=wrapper") {
        // For wrapper mode, Claude Code runs inside Cursor/VS Code
        if contents.contains("WRAPPER_IDE=cursor") {
            "cursor".to_string()
        } else if contents.contains("WRAPPER_IDE=code") {
            "code".to_string()
        } else {
            // Default to configured IDE wrapper name
            default_ide.to_string()
        }
    } else if let Some(line) = contents.lines().find(|l| l.starts_with("LAUNCH_IDE=")) {
        line.split('=').nth(1).unwrap_or(default_ide).to_string()
    } else {
        default_ide.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_cursor() {
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=cursor\nLAUNCH_IDE=claude";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_code() {
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=code\nLAUNCH_IDE=claude";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "code");
    }

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_default() {
        let contents = "LAUNCH_METHOD=wrapper\nLAUNCH_IDE=claude";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_file_contents_launch_ide() {
        let contents = "LAUNCH_IDE=cursor\nSOME_OTHER=value";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_launch_file_contents_empty() {
        let contents = "";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_file_contents_no_ide_info() {
        let contents = "SOME_KEY=value\nANOTHER_KEY=value2";
        let result = parse_launch_file_contents(contents, "default");
        assert_eq!(result, "default");
    }
}
