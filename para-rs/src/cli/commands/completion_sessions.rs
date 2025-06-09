use crate::config::Config;
use crate::core::session::SessionManager;
use crate::utils::Result;

pub fn execute() -> Result<()> {
    let config = Config::load_or_create()?;
    let session_manager = SessionManager::new(&config);
    
    match session_manager.list_sessions() {
        Ok(sessions) => {
            for session in sessions {
                println!("{}", session.name);
            }
        }
        Err(_) => {
            // Silent failure for completion compatibility
        }
    }
    
    Ok(())
}