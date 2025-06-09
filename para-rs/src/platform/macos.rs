use std::path::Path;
use std::process::Command;
use super::{PlatformManager, WindowInfo, IdeConfig};
use crate::utils::{Result, ParaError};

pub struct MacOSPlatform;

impl PlatformManager for MacOSPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        match ide_name.to_lowercase().as_str() {
            "cursor" => self.close_cursor_window(session_id),
            "code" | "vscode" => self.close_vscode_window(session_id),
            "claude" => self.close_claude_window(session_id),
            _ => self.generic_window_close(session_id, ide_name),
        }
    }
    
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        if config.wrapper_enabled {
            self.launch_wrapper_mode(config, path, prompt)
        } else {
            self.launch_standalone_ide(config, path, prompt)
        }
    }
    
    fn find_ide_windows(&self, session_pattern: &str) -> Result<Vec<WindowInfo>> {
        let script = format!(r#"
            set windowList to {{}}
            
            tell application "System Events"
                set allApps to (every application process whose visible is true)
                repeat with app in allApps
                    set appName to name of app
                    if appName contains "Cursor" or appName contains "Code" or appName contains "Terminal" then
                        try
                            set windows to (every window of app)
                            repeat with w in windows
                                set windowTitle to name of w
                                if windowTitle contains "{}" then
                                    set processID to unix id of app
                                    set windowID to id of w
                                    set windowList to windowList & {{appName, windowTitle, processID, windowID}}
                                end if
                            end repeat
                        end try
                    end if
                end repeat
            end tell
            
            return windowList
        "#, session_pattern);
        
        let output = self.execute_applescript_with_output(&script)?;
        self.parse_window_list(&output)
    }
    
    fn get_active_window_title(&self) -> Result<Option<String>> {
        let script = r#"
            tell application "System Events"
                try
                    set frontApp to first application process whose frontmost is true
                    set windowTitle to name of front window of frontApp
                    return windowTitle
                on error
                    return ""
                end try
            end tell
        "#;
        
        let output = self.execute_applescript_with_output(script)?;
        if output.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(output.trim().to_string()))
        }
    }
    
    fn bring_window_to_front(&self, window_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "System Events"
                try
                    set targetWindow to (first window whose id is {})
                    set frontmost of (first application process whose windows contains targetWindow) to true
                    perform action "AXRaise" of targetWindow
                on error
                    error "Window not found or cannot be brought to front"
                end try
            end tell
        "#, window_id);
        
        self.execute_applescript(&script)
    }
    
    fn terminate_process_group(&self, process_id: u32) -> Result<()> {
        let output = Command::new("pkill")
            .arg("-P")
            .arg(process_id.to_string())
            .output()?;
            
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to terminate process group {}: {}", process_id, error);
        }
        
        Ok(())
    }
}

impl MacOSPlatform {
    fn close_cursor_window(&self, session_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "Cursor"
                try
                    set windowList to every window
                    repeat with w in windowList
                        if name of w contains "{}" then
                            close w
                        end if
                    end repeat
                on error
                    -- Cursor might not be running or accessible
                end try
            end tell
        "#, session_id);
        
        self.execute_applescript(&script)
    }
    
    fn close_vscode_window(&self, session_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "Visual Studio Code"
                try
                    set windowList to every window
                    repeat with w in windowList
                        if name of w contains "{}" then
                            close w
                        end if
                    end repeat
                on error
                    -- VS Code might not be running or accessible
                end try
            end tell
        "#, session_id);
        
        self.execute_applescript(&script)
    }
    
    fn close_claude_window(&self, session_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "Terminal"
                try
                    set windowList to every window
                    repeat with w in windowList
                        if name of w contains "{}" then
                            close w
                        end if
                    end repeat
                on error
                    -- Terminal might not be running or accessible
                end try
            end tell
        "#, session_id);
        
        self.execute_applescript(&script)
    }
    
    fn generic_window_close(&self, session_id: &str, ide_name: &str) -> Result<()> {
        let script = format!(r#"
            tell application "{}"
                try
                    set windowList to every window
                    repeat with w in windowList
                        if name of w contains "{}" then
                            close w
                        end if
                    end repeat
                on error
                    -- Application might not be running or accessible
                end try
            end tell
        "#, ide_name, session_id);
        
        self.execute_applescript(&script)
    }
    
    pub fn launch_wrapper_mode(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        if config.name == "claude" {
            self.create_vscode_tasks_json(path, prompt)?;
        }
        
        let mut cmd = Command::new(&config.wrapper_command);
        cmd.arg(path);
        
        if config.wrapper_name == "cursor" {
            cmd.arg("--new-window");
        } else if config.wrapper_name == "code" {
            cmd.arg("--new-window");
        }
        
        cmd.spawn()?;
        
        if config.name == "claude" && config.wrapper_enabled {
            self.auto_start_claude_in_wrapper(path)?;
        }
        
        Ok(())
    }
    
    pub fn launch_standalone_ide(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        let mut cmd = Command::new(&config.command);
        cmd.arg(path);
        
        if let Some(prompt_text) = prompt {
            if config.name == "claude" {
                cmd.arg("--prompt").arg(prompt_text);
            }
        }
        
        if config.name == "cursor" || config.name == "code" {
            cmd.arg("--new-window");
        }
        
        cmd.spawn()?;
        Ok(())
    }
    
    pub fn create_vscode_tasks_json(&self, path: &Path, prompt: Option<&str>) -> Result<()> {
        let vscode_dir = path.join(".vscode");
        std::fs::create_dir_all(&vscode_dir)?;
        
        let tasks_json = if let Some(prompt) = prompt {
            format!(r#"{{
    "version": "2.0.0",
    "tasks": [
        {{
            "label": "Start Claude Code",
            "type": "shell",
            "command": "claude",
            "args": ["--prompt", "{}"],
            "group": "build",
            "presentation": {{
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "new"
            }},
            "runOptions": {{
                "runOn": "folderOpen"
            }}
        }}
    ]
}}"#, prompt.replace('"', r#"\""#))
        } else {
            r#"{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Start Claude Code",
            "type": "shell",
            "command": "claude",
            "group": "build",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "new"
            }
        }
    ]
}"#.to_string()
        };
        
        std::fs::write(vscode_dir.join("tasks.json"), tasks_json)?;
        Ok(())
    }
    
    fn auto_start_claude_in_wrapper(&self, _path: &Path) -> Result<()> {
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        let script = format!(r#"
            tell application "System Events"
                tell application process "Visual Studio Code"
                    keystroke "p" using {{command down, shift down}}
                    delay 0.5
                    keystroke "Tasks: Run Task"
                    delay 0.5
                    key code 36
                    delay 0.5
                    keystroke "Start Claude Code"
                    delay 0.5
                    key code 36
                end tell
            end tell
        "#);
        
        self.execute_applescript(&script).map_err(|_| {
            ParaError::platform_error("Failed to auto-start Claude Code in wrapper mode".to_string())
        })
    }
    
    fn execute_applescript(&self, script: &str) -> Result<()> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()?;
            
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ParaError::platform_error(format!("AppleScript failed: {}", error)));
        }
        
        Ok(())
    }
    
    pub fn execute_applescript_with_output(&self, script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()?;
            
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ParaError::platform_error(format!("AppleScript failed: {}", error)));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    fn parse_window_list(&self, output: &str) -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();
        
        for line in output.lines() {
            if let Some(window_info) = self.parse_window_line(line) {
                windows.push(window_info);
            }
        }
        
        Ok(windows)
    }
    
    pub fn parse_window_line(&self, line: &str) -> Option<WindowInfo> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 4 {
            Some(WindowInfo {
                app_name: parts[0].trim().to_string(),
                title: parts[1].trim().to_string(),
                process_id: parts[2].trim().parse().unwrap_or(0),
                id: parts[3].trim().to_string(),
            })
        } else {
            None
        }
    }
}