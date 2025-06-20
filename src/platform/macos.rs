use super::PlatformManager;
use crate::utils::Result;
use std::process::Command;

pub struct MacOSPlatform;

impl PlatformManager for MacOSPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Runtime check: This method should NEVER be called from tests
        // Tests should use mock IDE commands or cfg!(test) guards to prevent reaching this code
        if cfg!(test) {
            return Err(crate::utils::ParaError::ide_error(format!(
                "IDE window operations not supported in test environment. \
                 This indicates a test isolation failure. \
                 Session: {}, IDE: {}",
                session_id, ide_name
            )));
        }

        // Only works on macOS with osascript
        if Command::new("osascript").arg("--version").output().is_err() {
            return Ok(());
        }

        // Read launch file to determine actual IDE used (like legacy implementation)
        let state_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".para_state");
        let launch_file = state_dir.join(format!("{}.launch", session_id));

        let actual_ide = if launch_file.exists() {
            // Try to read the actual IDE used from launch file
            if let Ok(contents) = std::fs::read_to_string(&launch_file) {
                #[cfg(test)]
                {
                    Self::parse_launch_file_contents(&contents, ide_name)
                }
                #[cfg(not(test))]
                {
                    if contents.contains("LAUNCH_METHOD=wrapper") {
                        // For wrapper mode, Claude Code runs inside Cursor/VS Code
                        if contents.contains("WRAPPER_IDE=cursor") {
                            "cursor".to_string()
                        } else if contents.contains("WRAPPER_IDE=code") {
                            "code".to_string()
                        } else {
                            // Default to configured IDE wrapper name
                            ide_name.to_string()
                        }
                    } else if let Some(line) =
                        contents.lines().find(|l| l.starts_with("LAUNCH_IDE="))
                    {
                        line.split('=').nth(1).unwrap_or(ide_name).to_string()
                    } else {
                        ide_name.to_string()
                    }
                }
            } else {
                ide_name.to_string()
            }
        } else {
            ide_name.to_string()
        };

        let app_name = match actual_ide.to_lowercase().as_str() {
            "cursor" => "Cursor",
            "code" | "vscode" => "Code",
            _ => return Ok(()), // Only support Cursor and VS Code
        };

        // Different search strategies based on IDE (supports both Docker-style and timestamp formats)
        let search_fragment = {
            #[cfg(test)]
            {
                Self::format_search_fragment(session_id, &actual_ide)
            }
            #[cfg(not(test))]
            {
                if actual_ide == "cursor" {
                    // Check if it's a timestamp-based session (legacy format)
                    let timestamp_regex = regex::Regex::new(r"-\d{8}-\d{6}$").unwrap();
                    if timestamp_regex.is_match(session_id) {
                        // Legacy format: remove timestamp for Cursor window matching
                        // Cursor shows titles like "fish — session-name-20250607-123456"
                        timestamp_regex.replace(session_id, "").to_string()
                    } else {
                        // Docker-style format: use as-is (e.g., "eager_phoenix")
                        session_id.to_string()
                    }
                } else {
                    // VS Code shows full worktree directory name in title
                    session_id.to_string()
                }
            }
        };

        // Use AppleScript to close the window (matching legacy implementation exactly)
        let script = {
            #[cfg(test)]
            {
                Self::generate_applescript(app_name, &search_fragment)
            }
            #[cfg(not(test))]
            {
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
        };

        self.execute_applescript(&script)
    }
}

impl MacOSPlatform {
    fn execute_applescript(&self, script: &str) -> Result<()> {
        let output = Command::new("osascript").arg("-e").arg(script).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                eprintln!("Warning: AppleScript error: {}", stderr.trim());
            }
        }

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn parse_launch_file_contents(contents: &str, default_ide: &str) -> String {
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
    pub(crate) fn format_search_fragment(session_id: &str, ide_name: &str) -> String {
        if ide_name == "cursor" {
            // Check if it's a timestamp-based session (legacy format)
            let timestamp_regex = regex::Regex::new(r"-\d{8}-\d{6}$").unwrap();
            if timestamp_regex.is_match(session_id) {
                // Legacy format: remove timestamp for Cursor window matching
                // Cursor shows titles like "fish — session-name-20250607-123456"
                timestamp_regex.replace(session_id, "").to_string()
            } else {
                // Docker-style format: use as-is (e.g., "eager_phoenix")
                session_id.to_string()
            }
        } else {
            // VS Code shows full worktree directory name in title
            session_id.to_string()
        }
    }

    #[cfg(test)]
    pub(crate) fn generate_applescript(app_name: &str, search_fragment: &str) -> String {
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
}
