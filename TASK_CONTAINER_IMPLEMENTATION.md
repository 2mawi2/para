# Task: Implement Docker Container Support for Para Rust (SIMPLIFIED)

## Overview

Implement Docker container support for Para's Rust implementation, based on the successful shell implementation from the `container` branch. Keep it **as simple as possible** while providing the core functionality.

## Background Analysis

The shell implementation shows the **minimum viable approach**:

1. **1 main file** (`lib/para-docker.sh`) - ~1000 lines total
2. **Direct docker CLI** - Uses `docker run`, `docker exec`, `docker commit` 
3. **Simple auth** - Copy host `.claude` to container, commit as image
4. **Basic resource management** - Count containers with `docker ps | wc -l`
5. **3 integration points** - Modify existing commands, session, and IDE files

## SIMPLE Implementation Plan

### 1. Core Docker Module (1 file)
**File**: `src/core/docker.rs` (new)
**Size**: ~500-800 lines (similar to shell version)

**Key functions** (mirroring shell implementation):
```rust
pub fn check_docker_available() -> Result<()>
pub fn build_base_image() -> Result<()>  
pub fn setup_container_auth() -> Result<()>
pub fn create_container_session(session_id: &str, code_path: &Path) -> Result<()>
pub fn start_container(session_id: &str) -> Result<()>
pub fn pause_container(session_id: &str) -> Result<()>
pub fn resume_container(session_id: &str) -> Result<()>
pub fn cleanup_container(session_id: &str) -> Result<()>
pub fn count_containers() -> Result<usize>
```

**Implementation**: Use `std::process::Command` to call docker CLI directly (like shell version)
```rust
fn run_docker_command(args: &[&str]) -> Result<String> {
    let output = Command::new("docker")
        .args(args)
        .output()?;
    
    if !output.status.success() {
        bail!("Docker command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 2. Session Integration (modify existing)
**File**: `src/core/session/state.rs` (modify)
**Changes**: Add `SessionType::Container` enum variant

**File**: `src/core/session/manager.rs` (modify)  
**Changes**: When `container_mode_enabled()`, use container functions instead of worktree

### 3. CLI Integration (modify existing + 2 small files)
**File**: `src/cli/commands/mod.rs` (modify)
**Changes**: Add `pause`, `resume`, `container` commands

**File**: `src/cli/commands/pause.rs` (new - ~50 lines)
**File**: `src/cli/commands/resume.rs` (new - ~50 lines)

### 4. Configuration (modify existing)
**File**: `src/config/manager.rs` (modify)
**Changes**: Add `container.enabled: bool` field to config

No complex container config - just enable/disable flag

## Minimal File Changes

**Total new files**: 3 small files (~1000 lines total)
**Modified files**: 4 existing files (~100 lines of changes)

This is **10x simpler** than my original plan of 15+ files.

## Docker Operations (Direct CLI calls)

### Build Base Image
```rust
fn build_base_image() -> Result<()> {
    let dockerfile = format!(r#"
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y git curl nodejs npm sudo zsh
RUN npm install -g @anthropic-ai/claude-code
RUN useradd -m -s /bin/zsh para && echo "para ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers
WORKDIR /para-session
USER para
"#);
    
    let temp_file = write_temp_dockerfile(&dockerfile)?;
    run_docker_command(&["build", "-t", "para-base:latest", "-f", &temp_file, "."])?;
    Ok(())
}
```

### Authentication Setup
```rust
fn setup_container_auth() -> Result<()> {
    // 1. Create auth container
    run_docker_command(&[
        "run", "-dt", "--name", "para-auth-setup",
        "--network", "host", "para-base:latest", "sleep", "infinity"
    ])?;
    
    // 2. Copy host .claude to container
    if let Some(claude_dir) = dirs::home_dir().map(|h| h.join(".claude")) {
        run_docker_command(&["cp", &claude_dir.display().to_string(), "para-auth-setup:/home/para/.claude"])?;
    }
    
    // 3. Run Claude Code for auth (interactive)
    run_docker_command(&["exec", "-it", "para-auth-setup", "claude"])?;
    
    // 4. Commit authenticated image
    run_docker_command(&["commit", "para-auth-setup", "para-authenticated:latest"])?;
    
    // 5. Cleanup
    run_docker_command(&["rm", "-f", "para-auth-setup"])?;
    Ok(())
}
```

### Session Creation
```rust
fn create_container_session(session_id: &str, code_path: &Path) -> Result<()> {
    let container_name = format!("para-session-{}", session_id);
    let image = if authenticated_image_exists() { 
        "para-authenticated:latest" 
    } else { 
        "para-base:latest" 
    };
    
    run_docker_command(&[
        "run", "-dt", "--name", &container_name,
        "--mount", &format!("type=bind,source={},target=/para-session", code_path.display()),
        "--network", "host", image, "sleep", "infinity"
    ])?;
    Ok(())
}
```

### Resource Management
```rust
fn count_containers() -> Result<usize> {
    let output = run_docker_command(&["ps", "--filter", "name=para-session-", "--format", "{{.Names}}"])?;
    Ok(output.lines().count())
}

fn check_container_limit() -> Result<()> {
    if count_containers()? >= 5 {
        bail!("Maximum 5 containers reached. Use 'para pause' or 'para cancel' to free resources.");
    }
    Ok(())
}
```

## Integration Points

### Session Manager Integration
```rust
// In session/manager.rs
impl SessionManager {
    pub fn create_session(&self, name: &str, prompt: Option<&str>) -> Result<()> {
        if self.config.container.enabled {
            docker::check_container_limit()?;
            docker::create_container_session(name, &self.get_session_path(name))?;
            docker::start_container(name)?;
        } else {
            // Existing worktree logic
        }
        Ok(())
    }
    
    pub fn finish_session(&self, name: &str) -> Result<()> {
        if self.is_container_session(name) {
            docker::cleanup_container(name)?;
        } else {
            // Existing worktree cleanup
        }
        Ok(())
    }
}
```

### CLI Integration
```rust
// In cli/commands/pause.rs
pub fn execute_pause(session_name: Option<String>) -> Result<()> {
    let session_name = session_name.unwrap_or_else(|| auto_detect_session());
    
    if is_container_session(&session_name) {
        docker::pause_container(&session_name)?;
        println!("✅ Container session paused: {}", session_name);
    } else {
        bail!("Pause only works with container sessions");
    }
    Ok(())
}
```

## Success Criteria (Simplified)

1. ✅ **Basic container sessions work** - dispatch, start, finish, cancel
2. ✅ **Authentication works** - one-time setup, persistent auth
3. ✅ **Resource limits work** - max 5 containers
4. ✅ **Pause/resume works** - save/restore containers  
5. ✅ **Clean error handling** - helpful Docker error messages

## What This AVOIDS (Simplicity)

❌ **No complex API libraries** - Direct docker CLI calls
❌ **No separate auth manager** - Simple functions in main module
❌ **No complex image management** - Basic commit/tag operations
❌ **No resource monitoring** - Simple container counting
❌ **No async/await complexity** - Synchronous operations

## Implementation Order

1. **Core docker module** - Basic container operations (~500 lines)
2. **Session integration** - Modify existing session manager (~50 lines)
3. **CLI commands** - Add pause/resume commands (~100 lines)
4. **Configuration** - Add container.enabled flag (~20 lines)

**Total**: ~670 lines of new/modified code vs 15+ files and 2000+ lines in original plan

## Final Instructions

**Keep it simple**: Mirror the shell implementation as closely as possible
**Use direct docker CLI**: No complex libraries, just `std::process::Command`
**Minimal files**: 3 new files, 4 modified files maximum
**Focus on core functionality**: Container sessions, auth, pause/resume, limits

**Testing**: 
- Run `just test` to ensure all tests pass
- Test basic container workflow: dispatch → pause → resume → finish
- Test authentication setup works
- Test resource limits prevent >5 containers

**Completion**: Call `para finish 'Add Docker container support with pause/resume'` when done