use super::{launch_file::LaunchFileParser, PlatformManager};
use crate::utils::Result;
use std::process::Command;

pub struct MacOSPlatform;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub original_id: String,
}

pub trait IdeHandler {
    fn close_window(&self, session_info: &SessionInfo) -> Result<()>;
    fn generate_applescript(&self, session_info: &SessionInfo) -> String;
}

pub struct CursorHandler;

impl IdeHandler for CursorHandler {
    fn close_window(&self, session_info: &SessionInfo) -> Result<()> {
        if cfg!(test) {
            panic!(
                "CRITICAL: CursorHandler.close_window called from test environment! \
                 This indicates a test isolation failure. \
                 Session: {}",
                session_info.original_id
            );
        }
        let script = self.generate_applescript(session_info);
        execute_applescript(&script)
    }

    fn generate_applescript(&self, session_info: &SessionInfo) -> String {
        // Use original_id for collision-safe window matching, consistent with VSCode
        generate_applescript_template("Cursor", &session_info.original_id)
    }
}

pub struct VSCodeHandler;

impl IdeHandler for VSCodeHandler {
    fn close_window(&self, session_info: &SessionInfo) -> Result<()> {
        if cfg!(test) {
            panic!(
                "CRITICAL: VSCodeHandler.close_window called from test environment! \
                 This indicates a test isolation failure. \
                 Session: {}",
                session_info.original_id
            );
        }
        let script = self.generate_applescript(session_info);
        execute_applescript(&script)
    }

    fn generate_applescript(&self, session_info: &SessionInfo) -> String {
        // VS Code shows full worktree directory name in title
        let search_fragment = &session_info.original_id;
        generate_applescript_template("Code", search_fragment)
    }
}

fn execute_applescript(script: &str) -> Result<()> {
    if cfg!(test) {
        panic!(
            "CRITICAL: execute_applescript called from test environment! \
             This indicates a test isolation failure."
        );
    }

    let output = Command::new("osascript").arg("-e").arg(script).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("Warning: AppleScript error: {}", stderr.trim());
        }
    }

    Ok(())
}

pub(crate) fn generate_applescript_template(app_name: &str, search_fragment: &str) -> String {
    format!(
        r#"
on run argv
  set appName to "{app_name}"
  set windowTitleFragment to "{search_fragment}"
  
  log "AppleScript started for app: " & appName & " with title fragment: " & windowTitleFragment
  
  tell application "System Events"
    if not (exists process appName) then
      log "Error: Application process '" & appName & "' is not running."
      return "Application not running."
    end if
    
    tell process appName
      try
        set targetWindows to (every window whose name contains windowTitleFragment)
      on error errMsg
        log "Error: Could not get windows from " & appName & ". " & errMsg
        return "Error getting windows."
      end try

      if (count of targetWindows) is 0 then
        log "Failure: No window found with title containing '" & windowTitleFragment & "'"
        return "No matching window found."
      end if
      
      set targetWindow to item 1 of targetWindows
      
      log "Success: Found window: '" & (name of targetWindow) & "'"
      
      perform action "AXRaise" of targetWindow
      delay 0.2
      
      try
        click (button 1 of targetWindow)
        return "Successfully sent close command to window."
      on error
         log "Error: Could not click the close button. The window may not be standard."
         return "Could not click close button."
      end try

    end tell
  end tell
end run
        "#,
        app_name = app_name,
        search_fragment = search_fragment
    )
}

impl PlatformManager for MacOSPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Runtime check: This method should NEVER be called from tests
        // Tests should use mock IDE commands or cfg!(test) guards to prevent reaching this code
        if cfg!(test) {
            panic!(
                "CRITICAL: close_ide_window called from test environment! \
                 This indicates a test isolation failure. \
                 Session: {}, IDE: {}",
                session_id, ide_name
            );
        }

        // Only works on macOS with osascript
        if Command::new("osascript").arg("--version").output().is_err() {
            return Ok(());
        }

        // Determine the actual IDE used by reading the launch file
        let actual_ide = self.determine_actual_ide(session_id, ide_name)?;

        // Parse session information
        let session_info = self.parse_session_info(session_id)?;

        // Get the appropriate IDE handler and close the window
        let ide_handler = self.get_ide_handler(&actual_ide)?;
        ide_handler.close_window(&session_info)
    }
}

impl MacOSPlatform {
    fn determine_actual_ide(&self, session_id: &str, ide_name: &str) -> Result<String> {
        // Read launch file to determine actual IDE used (like legacy implementation)
        let state_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".para_state");
        let launch_file = state_dir.join(format!("{}.launch", session_id));

        let actual_ide = if launch_file.exists() {
            // Try to read the actual IDE used from launch file
            if let Ok(contents) = std::fs::read_to_string(&launch_file) {
                LaunchFileParser::parse_ide_from_contents(&contents, ide_name)
            } else {
                ide_name.to_string()
            }
        } else {
            ide_name.to_string()
        };

        Ok(actual_ide)
    }

    pub(crate) fn get_ide_handler(&self, ide_name: &str) -> Result<Box<dyn IdeHandler>> {
        match ide_name.to_lowercase().as_str() {
            "cursor" => Ok(Box::new(CursorHandler)),
            "code" | "vscode" => Ok(Box::new(VSCodeHandler)),
            _ => Err(crate::utils::ParaError::ide_error(format!(
                "Unsupported IDE: {}",
                ide_name
            ))),
        }
    }

    pub(crate) fn parse_session_info(&self, session_id: &str) -> Result<SessionInfo> {
        Ok(SessionInfo {
            original_id: session_id.to_string(),
        })
    }
}
