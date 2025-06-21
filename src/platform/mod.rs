pub mod launch_parser;
pub mod macos;
#[cfg(test)]
mod tests;

use crate::utils::Result;

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
