/// Shared launch file parser for extracting IDE information from launch file contents
pub struct LaunchFileParser;

impl LaunchFileParser {
    /// Parse IDE name from launch file contents
    ///
    /// This function handles both wrapper mode and direct launch IDE specification:
    /// - LAUNCH_METHOD=wrapper: Uses WRAPPER_IDE value (cursor/code)
    /// - LAUNCH_IDE=value: Uses specified IDE value
    /// - Default: Falls back to provided default_ide
    pub fn parse_ide_from_contents(contents: &str, default_ide: &str) -> String {
        if contents.contains("LAUNCH_METHOD=wrapper") {
            Self::parse_wrapper_mode(contents, default_ide)
        } else if let Some(line) = contents.lines().find(|l| l.starts_with("LAUNCH_IDE=")) {
            line.split('=').nth(1).unwrap_or(default_ide).to_string()
        } else {
            default_ide.to_string()
        }
    }

    /// Parse wrapper mode configuration from launch file contents
    ///
    /// In wrapper mode, Claude Code runs inside Cursor/VS Code terminal
    fn parse_wrapper_mode(contents: &str, default_ide: &str) -> String {
        if contents.contains("WRAPPER_IDE=cursor") {
            "cursor".to_string()
        } else if contents.contains("WRAPPER_IDE=code") {
            "code".to_string()
        } else {
            // Default to configured IDE wrapper name
            default_ide.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wrapper_mode_cursor() {
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=cursor\nLAUNCH_IDE=claude";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_wrapper_mode_code() {
        let contents = "LAUNCH_METHOD=wrapper\nWRAPPER_IDE=code\nLAUNCH_IDE=claude";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "code");
    }

    #[test]
    fn test_parse_wrapper_mode_default() {
        let contents = "LAUNCH_METHOD=wrapper\nLAUNCH_IDE=claude";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_launch_ide() {
        let contents = "LAUNCH_IDE=cursor\nSOME_OTHER=value";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_parse_empty_contents() {
        let contents = "";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_parse_no_ide_info() {
        let contents = "SOME_KEY=value\nANOTHER_KEY=value2";
        let result = LaunchFileParser::parse_ide_from_contents(contents, "default");
        assert_eq!(result, "default");
    }
}
