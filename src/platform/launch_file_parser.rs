/// Shared utility for parsing IDE information from launch file contents
/// Eliminates duplication between macos.rs and tests.rs
pub fn parse_ide_from_launch_contents(contents: &str, default_ide: &str) -> String {
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
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_code() {
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=code\nLAUNCH_IDE=claude";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "code");
    }

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_default() {
        let contents = "LAUNCH_METHOD=wrapper\nLAUNCH_IDE=claude";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_file_contents_launch_ide() {
        let contents = "LAUNCH_IDE=cursor\nSOME_OTHER=value";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_launch_file_contents_empty() {
        let contents = "";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_file_contents_no_ide_info() {
        let contents = "SOME_KEY=value\nANOTHER_KEY=value2";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_file_contents_launch_ide_priority() {
        // LAUNCH_IDE should take precedence when not in wrapper mode
        let contents = "LAUNCH_IDE=code\nSOME_OTHER=value";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "code");
    }

    #[test]
    fn test_parse_launch_file_contents_wrapper_mode_with_both() {
        // In wrapper mode, WRAPPER_IDE should take precedence over LAUNCH_IDE
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=cursor\nLAUNCH_IDE=code";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_launch_file_contents_invalid_launch_ide() {
        // Test with LAUNCH_IDE= (empty value)
        let contents = "LAUNCH_IDE=";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_launch_file_contents_multiline_with_noise() {
        let contents =
            "# Comment line\nSOME_CONFIG=value\nLAUNCH_IDE=cursor\nANOTHER_CONFIG=value2";
        let result = parse_ide_from_launch_contents(contents, "default");
        assert_eq!(result, "cursor");
    }
}
