# Windows Support Investigation for Para

## Executive Summary

Para can be extended to support Windows with minimal architectural changes. The primary blocker is the AppleScript-based IDE window automation in the macOS platform module, which can be replaced with PowerShell + Windows API automation on Windows.

## Current State Analysis

### Platform-Specific Code Location
- **Main blocker**: `src/platform/macos.rs` (142-line AppleScript implementation)
- **Platform abstraction**: `src/platform/mod.rs` (already supports cross-platform design)
- **Generic fallback**: Already exists for non-macOS platforms

### Key Dependencies
- **AppleScript dependency**: Uses `osascript` command for IDE window automation
- **Cross-platform ready**: Git operations, session management, and IDE launching are platform-agnostic
- **Rust dependencies**: All current dependencies support Windows

## Technical Analysis

### AppleScript Functionality (src/platform/macos.rs:68-141)
The macOS implementation uses AppleScript to:
1. Find IDE windows by partial title match (session ID)
2. Focus and raise the target window
3. Send close command via accessibility API
4. Handle different IDE-specific window title patterns

### Current Platform Architecture
```rust
pub trait PlatformManager {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()>;
}

// Platform selection
#[cfg(target_os = "macos")]
return Box::new(macos::MacOSPlatform);

#[cfg(not(target_os = "macos"))]
return Box::new(GenericPlatform); // No-op implementation
```

## Windows Implementation Strategy

### Recommended Hybrid Approach

#### Primary Method: PowerShell Integration
```powershell
Get-Process -Name "Code" -ErrorAction SilentlyContinue | 
Where-Object { $_.MainWindowTitle -like "*session_id*" } | 
ForEach-Object { $_.CloseMainWindow() }
```

**Advantages:**
- Built into Windows 10/11
- Reliable window detection and closure
- No additional dependencies
- Handles partial title matching well

#### Fallback Method: Windows API
```rust
use windows::{
    Win32::UI::WindowsAndMessaging::{FindWindowW, SendMessageW, WM_CLOSE},
    Win32::Foundation::HWND,
};
```

**Advantages:**
- Direct system calls
- More robust when PowerShell unavailable
- Precise window targeting

### Implementation Plan

#### Phase 1: Create Windows Platform Module
Create `src/platform/windows.rs`:

```rust
use super::PlatformManager;
use crate::utils::Result;
use std::process::Command;

pub struct WindowsPlatform;

impl PlatformManager for WindowsPlatform {
    fn close_ide_window(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Try PowerShell first (most reliable)
        if let Ok(_) = self.close_via_powershell(session_id, ide_name) {
            return Ok(());
        }
        
        // Fallback to Windows API if available
        #[cfg(target_os = "windows")]
        {
            self.close_via_windows_api(session_id, ide_name)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(())
        }
    }
}

impl WindowsPlatform {
    fn close_via_powershell(&self, session_id: &str, ide_name: &str) -> Result<()> {
        let process_name = match ide_name.to_lowercase().as_str() {
            "cursor" => "Cursor",
            "code" | "vscode" => "Code",
            _ => return Ok(()),
        };
        
        // Read launch file to determine actual IDE used (same as macOS)
        let actual_ide = self.determine_actual_ide(session_id, ide_name)?;
        
        let script = format!(
            r#"
            Get-Process -Name "{}" -ErrorAction SilentlyContinue | 
            Where-Object {{ $_.MainWindowTitle -like "*{}*" }} | 
            ForEach-Object {{ 
                Write-Host "Closing window: $($_.MainWindowTitle)"
                $_.CloseMainWindow() 
            }}
            "#,
            process_name, session_id
        );
        
        let output = Command::new("powershell")
            .args(["-Command", &script])
            .output()?;
            
        if !output.status.success() {
            eprintln!("PowerShell script failed: {}", 
                String::from_utf8_lossy(&output.stderr));
        }
        
        Ok(())
    }
    
    fn determine_actual_ide(&self, session_id: &str, ide_name: &str) -> Result<String> {
        // Same logic as macOS implementation for reading .launch files
        // This functionality is already cross-platform
        Ok(ide_name.to_string())
    }
    
    #[cfg(target_os = "windows")]
    fn close_via_windows_api(&self, session_id: &str, ide_name: &str) -> Result<()> {
        // Windows API implementation as fallback
        // Uses FindWindowW + SendMessageW with WM_CLOSE
        Ok(())
    }
}
```

#### Phase 2: Update Platform Manager
Update `src/platform/mod.rs`:

```rust
#[cfg(target_os = "windows")]
pub mod windows;

pub fn get_platform_manager() -> Box<dyn PlatformManager> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSPlatform);
    
    #[cfg(target_os = "windows")]  
    return Box::new(windows::WindowsPlatform);
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Box::new(GenericPlatform);
}
```

#### Phase 3: Dependencies
Update `Cargo.toml`:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_Foundation"
]}
```

## IDE Integration Details

### Window Title Patterns
- **VS Code**: Shows full worktree directory name in title
- **Cursor**: Shows session name with timestamp format

### Process Names
- **VS Code**: "Code.exe" 
- **Cursor**: "Cursor.exe"

### Launch File Compatibility
The existing `.para/state/*.launch` file approach works on Windows:
- Same file format and parsing logic
- Cross-platform path handling already implemented
- Wrapper mode detection remains the same

## Testing Strategy

### Test Environment Setup
- Windows 10/11 with PowerShell 5.1+
- VS Code and/or Cursor installed
- Rust toolchain with Windows targets

### Test Cases
1. **PowerShell availability test**: Verify `powershell` command works
2. **Window detection test**: Find IDE windows by partial title
3. **Window closing test**: Successfully close target windows
4. **Fallback test**: Windows API when PowerShell fails
5. **Cross-platform test**: Ensure non-Windows platforms unaffected

## Implementation Effort

### Code Changes Required
- **New file**: `src/platform/windows.rs` (~100 lines)
- **Modified file**: `src/platform/mod.rs` (~5 lines added)
- **Modified file**: `Cargo.toml` (~4 lines added)

### Risk Assessment
- **Low risk**: Existing architecture supports this change
- **Minimal impact**: Windows-specific code is isolated
- **Graceful degradation**: Falls back to no-op if automation fails

## Alternative Approaches Considered

### 1. Cross-Platform GUI Automation Crates
- **Enigo**: Limited to input simulation, not window management
- **Tauri**: Overkill for Para's specific needs
- **Verdict**: Not suitable for Para's window automation requirements

### 2. Pure Windows API Approach
- **Pros**: Most direct, no PowerShell dependency
- **Cons**: More complex implementation, requires more Windows-specific code
- **Verdict**: Good as fallback, not primary approach

### 3. Alternative Command-Line Tools
- **PowerShell**: ✅ Built into Windows, reliable
- **WMIC**: Deprecated in favor of PowerShell
- **Taskkill**: Too aggressive, kills entire process
- **Verdict**: PowerShell is the best option

## Conclusion

Windows support for Para is highly feasible with the proposed hybrid PowerShell + Windows API approach. The implementation would:

1. **Require minimal code changes** (~100 lines total)
2. **Maintain existing architecture** (no breaking changes)
3. **Provide reliable window automation** (PowerShell primary, API fallback)
4. **Add single conditional dependency** (Windows crate for Windows targets only)
5. **Preserve cross-platform compatibility** (existing non-Windows behavior unchanged)

The modular platform design of Para makes this extension straightforward, with the primary work being the translation of AppleScript functionality to PowerShell + Windows API equivalents.