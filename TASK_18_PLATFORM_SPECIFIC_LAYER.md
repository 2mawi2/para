# Task 18: Platform-Specific Layer for IDE Window Management

## Objective
Implement the platform-specific layer for advanced IDE integration, focusing on automatic window management, process control, and platform-native IDE interactions.

## Background
The PRD specifies a `src/platform/` module with platform-specific implementations for macOS, Linux, and Windows. This is currently completely missing from the Rust implementation, but it's essential for professional IDE integration features like automatic window closing, wrapper mode support, and conflict resolution workflows.

## Requirements

### 1. Platform Module Architecture
Implement the platform layer as specified in PRD Section 4.1:

```rust
// src/platform/mod.rs
pub mod macos;
pub mod linux;
pub mod windows;

use std::path::Path;
use crate::utils::Result;

pub trait PlatformManager {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()>;
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()>;
    fn find_ide_windows(&self, session_pattern: &str) -> Result<Vec<WindowInfo>>;
    fn get_active_window_title(&self) -> Result<Option<String>>;
    fn bring_window_to_front(&self, window_id: &str) -> Result<()>;
    fn terminate_process_group(&self, process_id: u32) -> Result<()>;
}

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
```

### 2. macOS Implementation
Create comprehensive macOS support using AppleScript and native APIs:

```rust
// src/platform/macos.rs
use std::process::Command;
use super::{PlatformManager, WindowInfo, IdeConfig};

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
}

impl MacOSPlatform {
    fn close_cursor_window(&self, session_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "Cursor"
                set windowList to every window
                repeat with w in windowList
                    if name of w contains "{}" then
                        close w
                    end if
                end repeat
            end tell
        "#, session_id);
        
        self.execute_applescript(&script)
    }
    
    fn close_vscode_window(&self, session_id: &str) -> Result<()> {
        let script = format!(r#"
            tell application "Visual Studio Code"
                set windowList to every window
                repeat with w in windowList
                    if name of w contains "{}" then
                        close w
                    end if
                end repeat
            end tell
        "#, session_id);
        
        self.execute_applescript(&script)
    }
    
    fn launch_wrapper_mode(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        // Create .vscode/tasks.json for Claude Code integration
        if config.name == "claude" {
            self.create_vscode_tasks_json(path, prompt)?;
        }
        
        // Launch wrapper IDE
        let mut cmd = Command::new(&config.wrapper_command);
        cmd.arg(path);
        
        if config.wrapper_name == "cursor" {
            cmd.arg("--new-window");
        } else if config.wrapper_name == "code" {
            cmd.arg("--new-window");
        }
        
        cmd.spawn()?;
        
        // Auto-start Claude Code if configured
        if config.name == "claude" && config.wrapper_enabled {
            self.auto_start_claude_in_wrapper(path)?;
        }
        
        Ok(())
    }
    
    fn create_vscode_tasks_json(&self, path: &Path, prompt: Option<&str>) -> Result<()> {
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
    
    fn execute_applescript(&self, script: &str) -> Result<()> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()?;
            
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(crate::utils::ParaError::platform_error(format!("AppleScript failed: {}", error)));
        }
        
        Ok(())
    }
}
```

### 3. Linux Implementation
Implement Linux support using X11/Wayland tools:

```rust
// src/platform/linux.rs
use super::{PlatformManager, WindowInfo, IdeConfig};

pub struct LinuxPlatform;

impl PlatformManager for LinuxPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Use wmctrl or xdotool for X11
        if self.is_wayland() {
            self.close_window_wayland(session_id, ide_name)
        } else {
            self.close_window_x11(session_id, ide_name)
        }
    }
    
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        // Similar to macOS but using Linux-specific approaches
        self.launch_linux_ide(config, path, prompt)
    }
}

impl LinuxPlatform {
    fn is_wayland(&self) -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }
    
    fn close_window_x11(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Use wmctrl to find and close windows
        let output = Command::new("wmctrl")
            .arg("-l")
            .output()?;
            
        let window_list = String::from_utf8_lossy(&output.stdout);
        
        for line in window_list.lines() {
            if line.contains(session_id) && line.contains(ide_name) {
                let window_id = line.split_whitespace().next().unwrap();
                Command::new("wmctrl")
                    .arg("-i")
                    .arg("-c")
                    .arg(window_id)
                    .output()?;
            }
        }
        
        Ok(())
    }
    
    fn close_window_wayland(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Wayland doesn't allow direct window manipulation
        // Fall back to process-based closing
        self.close_by_process_name(ide_name, session_id)
    }
}
```

### 4. Windows Implementation
Implement Windows support using Win32 APIs:

```rust
// src/platform/windows.rs
use super::{PlatformManager, WindowInfo, IdeConfig};

pub struct WindowsPlatform;

impl PlatformManager for WindowsPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Use Windows API to find and close windows
        self.close_windows_window(session_id, ide_name)
    }
    
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()> {
        self.launch_windows_ide(config, path, prompt)
    }
}

impl WindowsPlatform {
    fn close_windows_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Use PowerShell or taskkill for window management
        let script = format!(r#"
            Get-Process | Where-Object {{$_.MainWindowTitle -like "*{}*" -and $_.ProcessName -like "*{}*"}} | Stop-Process -Force
        "#, session_id, ide_name);
        
        Command::new("powershell")
            .arg("-Command")
            .arg(script)
            .output()?;
            
        Ok(())
    }
}
```

### 5. Platform Factory and Integration
Create platform detection and factory:

```rust
// src/platform/mod.rs (continued)
pub fn get_platform_manager() -> Box<dyn PlatformManager> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSPlatform);
    
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxPlatform);
    
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsPlatform);
    
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return Box::new(GenericPlatform);
}

pub struct GenericPlatform;

impl PlatformManager for GenericPlatform {
    fn close_ide_window(&self, _session_id: &str, _ide_name: &str) -> Result<()> {
        // No-op for unsupported platforms
        eprintln!("Warning: IDE window management not supported on this platform");
        Ok(())
    }
    
    fn launch_ide_with_wrapper(&self, config: &IdeConfig, path: &Path, _prompt: Option<&str>) -> Result<()> {
        // Fall back to basic IDE launch
        Command::new(&config.command)
            .arg(path)
            .spawn()?;
        Ok(())
    }
}
```

### 6. Integration with Core Commands
Update commands to use platform manager:

```rust
// In finish/cancel/integrate commands
use crate::platform::get_platform_manager;

pub fn execute_finish(args: FinishArgs) -> Result<()> {
    // ... existing logic ...
    
    // Close IDE window after successful finish
    let platform = get_platform_manager();
    if let Err(e) = platform.close_ide_window(&session.id, &config.ide.name) {
        eprintln!("Warning: Failed to close IDE window: {}", e);
    }
    
    // ... rest of finish logic ...
}
```

## Implementation Details

### Files to Create
1. `para-rs/src/platform/mod.rs` - Platform module and traits
2. `para-rs/src/platform/macos.rs` - macOS-specific implementation
3. `para-rs/src/platform/linux.rs` - Linux-specific implementation  
4. `para-rs/src/platform/windows.rs` - Windows-specific implementation
5. Update `para-rs/src/lib.rs` to include platform module

### Dependencies to Add
Add to `Cargo.toml`:
```toml
[target.'cfg(target_os = "macos")'.dependencies]
# macOS-specific dependencies if needed

[target.'cfg(target_os = "linux")'.dependencies]
# Linux-specific dependencies if needed

[target.'cfg(target_os = "windows")'.dependencies]
# Windows-specific dependencies if needed
```

### Testing Strategy
1. **Unit tests** for each platform implementation
2. **Integration tests** with mock IDEs
3. **Manual testing** on each target platform
4. **Fallback testing** for unsupported scenarios
5. **Error handling tests** for permission issues

### Error Handling
Handle platform-specific errors gracefully:
- Permission denied for window management
- Missing system tools (wmctrl, osascript, etc.)
- Unsupported desktop environments
- IDE not installed or not running

### Security Considerations
- Validate window titles to prevent injection attacks
- Sanitize AppleScript inputs
- Use safe process termination methods
- Handle permission requests appropriately

## Legacy Reference
Study the legacy implementation:
- `lib/para-ide.sh` for IDE integration patterns
- Shell script IDE closing logic
- AppleScript usage in legacy implementation
- Window management strategies

## Validation Criteria
- [ ] Platform detection works correctly on all target platforms
- [ ] IDE window closing works for Cursor, VS Code, and Claude Code
- [ ] Wrapper mode launches IDEs correctly with task generation
- [ ] .vscode/tasks.json creation works for Claude Code integration
- [ ] Error handling is graceful for unsupported scenarios
- [ ] Performance impact is minimal
- [ ] All tests pass on target platforms
- [ ] Legacy functionality is preserved and enhanced

## Completion
When complete, call `para finish "Implement platform-specific layer for advanced IDE window management"` to finish the task.

The agent should focus on getting macOS implementation working first (as it has the most sophisticated AppleScript integration), then Linux, then Windows. Each platform should have comprehensive error handling and graceful degradation.