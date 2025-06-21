pub mod macos;
#[cfg(test)]
mod tests;

use crate::utils::Result;

/// Shared utility for parsing launch file contents across platform implementations.
/// This eliminates code duplication between production and test code.
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

pub trait PlatformManager {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()>;
}

pub fn get_platform_manager() -> Box<dyn PlatformManager> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSPlatform);

    #[cfg(not(target_os = "macos"))]
    return Box::new(GenericPlatform);
}

pub struct GenericPlatform;

impl PlatformManager for GenericPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Runtime check: This method should NEVER be called from tests
        if cfg!(test) {
            panic!(
                "CRITICAL: close_ide_window called from test environment! \
                 This indicates a test isolation failure. \
                 Session: {}, IDE: {}",
                session_id, ide_name
            );
        }

        // IDE window closing only supported on macOS
        Ok(())
    }
}
