pub mod macos;
#[cfg(test)]
mod tests;

use std::path::Path;
use crate::utils::Result;

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub process_id: u32,
    pub app_name: String,
}

#[derive(Debug, Clone)]
pub struct IdeConfig {
    pub name: String,
    pub command: String,
    pub wrapper_enabled: bool,
    pub wrapper_name: String,
    pub wrapper_command: String,
}

pub trait PlatformManager {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()>;
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()>;
    fn find_ide_windows(&self, session_pattern: &str) -> Result<Vec<WindowInfo>>;
    fn get_active_window_title(&self) -> Result<Option<String>>;
    fn bring_window_to_front(&self, window_id: &str) -> Result<()>;
    fn terminate_process_group(&self, process_id: u32) -> Result<()>;
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
        eprintln!("Warning: Para only supports macOS. IDE window management is not available on this platform.");
        Ok(())
    }
    
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, _prompt: Option<&str>) -> Result<()> {
        eprintln!("Warning: Para only supports macOS. Attempting basic IDE launch...");
        std::process::Command::new(&config.command)
            .arg(path)
            .spawn()?;
        Ok(())
    }
    
    fn find_ide_windows(&self, _session_pattern: &str) -> Result<Vec<WindowInfo>> {
        Ok(Vec::new())
    }
    
    fn get_active_window_title(&self) -> Result<Option<String>> {
        Ok(None)
    }
    
    fn bring_window_to_front(&self, _window_id: &str) -> Result<()> {
        Ok(())
    }
    
    fn terminate_process_group(&self, _process_id: u32) -> Result<()> {
        Ok(())
    }
}