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
    fn close_ide_window(&self, _session_id: &str, _ide_name: &str) -> Result<()> {
        // IDE window closing only supported on macOS
        Ok(())
    }
}