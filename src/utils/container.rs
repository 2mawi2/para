//! Container detection utilities

use std::env;
use std::path::Path;

/// Check if we're running inside a Docker container
pub fn is_inside_container() -> bool {
    // Check multiple indicators of container environment

    // Check for Docker environment file
    if Path::new("/.dockerenv").exists() {
        return true;
    }

    // Check for Para container environment variable
    if env::var("PARA_CONTAINER").is_ok() {
        return true;
    }

    // Check for common container environment variables
    if env::var("CONTAINER").is_ok() {
        return true;
    }

    false
}

/// Get the current session name from container environment
pub fn get_container_session() -> Option<String> {
    env::var("PARA_SESSION").ok()
}

/// Check if we're running inside a Para container specifically
#[allow(dead_code)]
pub fn is_para_container() -> bool {
    env::var("PARA_CONTAINER").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_container_detection_with_dockerenv() {
        // This test would need a way to mock the .dockerenv file
        // For now, just test that the function doesn't panic
        let _result = is_inside_container();
    }

    #[test]
    fn test_container_detection_with_env_var() {
        // Test PARA_CONTAINER environment variable
        env::set_var("PARA_CONTAINER", "1");
        assert!(is_inside_container());
        assert!(is_para_container());
        env::remove_var("PARA_CONTAINER");

        // Test CONTAINER environment variable
        env::set_var("CONTAINER", "1");
        assert!(is_inside_container());
        assert!(!is_para_container());
        env::remove_var("CONTAINER");
    }

    #[test]
    fn test_get_container_session() {
        // Test with session name set
        env::set_var("PARA_SESSION", "test-session");
        assert_eq!(get_container_session(), Some("test-session".to_string()));
        env::remove_var("PARA_SESSION");

        // Test without session name
        assert_eq!(get_container_session(), None);
    }
}
