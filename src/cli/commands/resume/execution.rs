use crate::config::Config;
use crate::core::ide::IdeManager;
use crate::utils::Result;
use std::path::Path;

/// Launch the IDE for a specific session path
pub fn launch_ide_for_session(config: &Config, session_path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // Launch IDE with default options (no skip permissions, no continue conversation)
    ide_manager.launch(session_path, false)
}