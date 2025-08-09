# Para Terminal Mode Implementation Plan

## Overview
Implement a `--terminal` flag for `para start` and `para resume` commands that returns structured JSON output instead of launching VS Code. This enables external tools like para-ui to manage para sessions in their own terminal environments.

## Design Principles
1. **Minimal Changes**: Add terminal mode without affecting existing behavior
2. **Complete Information**: Return all data needed to start/resume a session
3. **AI Agnostic**: Structure supports Claude now but extensible to other AIs
4. **No Side Effects**: In terminal mode, only create worktree and return info, no IDE launch
5. **Explicit Limitations**: Terminal mode initially doesn't support sandboxing or docker

## Command Interface

### Start Command
```bash
para start [name] --terminal [--dangerously-skip-permissions] [-p prompt] [-f file]
```

Returns JSON with session setup information instead of launching IDE.

### Resume Command  
```bash
para resume [name] --terminal [--dangerously-skip-permissions] [-p prompt] [-f file]
```

Returns JSON with session resume information instead of launching IDE.

## JSON Output Structure

### Start Response
```json
{
  "success": true,
  "action": "start",
  "session": {
    "name": "feature-auth",
    "id": "feature-auth",
    "worktree_path": "/path/to/repo/.para/worktrees/feature-auth",
    "branch": "para/feature-auth",
    "base_branch": "main"
  },
  "terminal_config": {
    "working_directory": "/path/to/repo/.para/worktrees/feature-auth",
    "ai_command": "claude",
    "ai_args": ["--dangerously-skip-permissions"],
    "initial_prompt": "implement user authentication",
    "environment": {
      "PARA_SESSION": "feature-auth",
      "PARA_WORKTREE": "/path/to/repo/.para/worktrees/feature-auth"
    }
  },
  "metadata": {
    "created_at": "2024-01-15T10:30:00Z",
    "para_version": "1.0.0",
    "ai_backend": "claude"
  }
}
```

### Resume Response
```json
{
  "success": true,
  "action": "resume",
  "session": {
    "name": "feature-auth",
    "id": "feature-auth",
    "worktree_path": "/path/to/repo/.para/worktrees/feature-auth",
    "branch": "para/feature-auth",
    "base_branch": "main",
    "has_uncommitted_changes": false
  },
  "terminal_config": {
    "working_directory": "/path/to/repo/.para/worktrees/feature-auth",
    "ai_command": "claude",
    "ai_args": ["--dangerously-skip-permissions", "-r", "session-id-123"],
    "resume_prompt": "continue with the task",
    "claude_session_id": "session-id-123",
    "environment": {
      "PARA_SESSION": "feature-auth",
      "PARA_WORKTREE": "/path/to/repo/.para/worktrees/feature-auth"
    }
  },
  "metadata": {
    "resumed_at": "2024-01-15T11:00:00Z",
    "last_activity": "2024-01-15T10:45:00Z",
    "para_version": "1.0.0",
    "ai_backend": "claude"
  }
}
```

### Error Response
```json
{
  "success": false,
  "error": {
    "code": "SESSION_EXISTS",
    "message": "Session 'feature-auth' already exists",
    "suggestion": "Use 'para resume feature-auth --terminal' to continue the session"
  }
}
```

## Implementation Details

### 1. Modified Start Flow
```rust
// In src/cli/commands/start.rs or unified_start.rs
pub fn execute_start(args: StartArgs, config: &Config) -> Result<()> {
    // Validate session name
    let session_name = determine_session_name(&args)?;
    
    // Check for terminal mode early
    if args.terminal {
        return execute_terminal_start(args, config);
    }
    
    // ... existing start logic for IDE mode
}

fn execute_terminal_start(args: StartArgs, config: &Config) -> Result<()> {
    // 1. Validate inputs
    validate_terminal_mode_args(&args)?;
    
    // 2. Create session (worktree, branch, etc.)
    let session = create_session_minimal(&args, config)?;
    
    // 3. Prepare prompt
    let prompt = resolve_prompt(&args)?;
    
    // 4. Build terminal config
    let terminal_config = build_terminal_config(
        &session,
        &prompt,
        args.dangerously_skip_permissions,
        config
    )?;
    
    // 5. Output JSON
    let response = StartResponse {
        success: true,
        action: "start",
        session: session.to_json(),
        terminal_config,
        metadata: build_metadata("start"),
    };
    
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
```

### 2. Modified Resume Flow
```rust
// In src/cli/commands/resume/mod.rs
pub fn execute_resume(args: ResumeArgs, config: &Config) -> Result<()> {
    if args.terminal {
        return execute_terminal_resume(args, config);
    }
    
    // ... existing resume logic
}

fn execute_terminal_resume(args: ResumeArgs, config: &Config) -> Result<()> {
    // 1. Find session
    let session = find_session(&args.session)?;
    
    // 2. Find Claude session ID for resume
    let claude_session_id = find_claude_session_id(&session.worktree_path)?;
    
    // 3. Prepare prompt if provided
    let prompt = resolve_prompt(&args)?;
    
    // 4. Build terminal config with resume info
    let terminal_config = build_resume_terminal_config(
        &session,
        claude_session_id,
        prompt,
        args.dangerously_skip_permissions,
        config
    )?;
    
    // 5. Output JSON
    let response = ResumeResponse {
        success: true,
        action: "resume",
        session: session.to_json(),
        terminal_config,
        metadata: build_metadata("resume"),
    };
    
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
```

### 3. Terminal Mode Validation
```rust
fn validate_terminal_mode_args(args: &StartArgs) -> Result<()> {
    // Check for unsupported combinations
    if args.terminal {
        if args.container {
            return Err(ParaError::validation(
                "Docker containers not supported in terminal mode yet"
            ));
        }
        
        if args.sandbox_args.sandbox {
            return Err(ParaError::validation(
                "Sandboxing not supported in terminal mode yet"
            ));
        }
        
        if args.sandbox_args.sandbox_no_network {
            return Err(ParaError::validation(
                "Network sandboxing not supported in terminal mode yet"
            ));
        }
    }
    
    Ok(())
}
```

### 4. Claude Session Detection
```rust
fn find_claude_session_id(worktree_path: &Path) -> Result<Option<String>> {
    // Reuse existing logic from resume/claude_session.rs
    match find_claude_session(worktree_path)? {
        Some(session) => Ok(Some(session.id)),
        None => Ok(None)
    }
}
```

### 5. Configuration Integration
```rust
fn build_terminal_config(
    session: &Session,
    prompt: Option<String>,
    skip_permissions: bool,
    config: &Config
) -> TerminalConfig {
    let mut ai_args = vec![];
    
    // Add base AI command (from config)
    let ai_command = config.ide.command.clone()
        .unwrap_or_else(|| "claude".to_string());
    
    // Add flags
    if skip_permissions {
        ai_args.push("--dangerously-skip-permissions".to_string());
    }
    
    // Build environment
    let mut environment = HashMap::new();
    environment.insert("PARA_SESSION".to_string(), session.name.clone());
    environment.insert("PARA_WORKTREE".to_string(), 
                      session.worktree_path.to_string_lossy().to_string());
    
    TerminalConfig {
        working_directory: session.worktree_path.clone(),
        ai_command,
        ai_args,
        initial_prompt: prompt,
        environment,
    }
}
```

## Edge Cases and Handling

### 1. Missing Claude Session on Resume
If no Claude session ID is found during resume in terminal mode:
```json
{
  "terminal_config": {
    "ai_command": "claude",
    "ai_args": ["--dangerously-skip-permissions", "-c"],
    "resume_prompt": "continue with the task",
    "claude_session_id": null,
    "note": "No Claude session found, will start new conversation with context"
  }
}
```
The client can decide whether to use `-c` (continue) or start fresh.

### 2. Session Already Exists
```bash
para start auth --terminal
# Error: Session exists
{
  "success": false,
  "error": {
    "code": "SESSION_EXISTS",
    "message": "Session 'auth' already exists",
    "suggestion": "Use 'para resume auth --terminal' to continue"
  }
}
```

### 3. Invalid Session Name
```bash
para start "my session!" --terminal
# Error: Invalid name
{
  "success": false,
  "error": {
    "code": "INVALID_NAME",
    "message": "Session name contains invalid characters",
    "suggestion": "Use only alphanumeric characters, hyphens, and underscores"
  }
}
```

### 4. Prompt Handling
- **File prompt**: Read file content and include in `initial_prompt`/`resume_prompt`
- **Inline prompt**: Pass directly
- **No prompt**: Set to null, client decides default behavior

### 5. Dangerous Permissions Flag
Always passed through to ai_args when specified:
- Start: `["--dangerously-skip-permissions"]`
- Resume: `["--dangerously-skip-permissions", "-r", "session-id"]`

## Migration Path

### Phase 1: Basic Implementation
1. Add `--terminal` flag to argument parsers
2. Implement JSON response structures
3. Add terminal mode execution paths
4. Return appropriate JSON for start/resume

### Phase 2: Enhanced Features
1. Add `--output-format` flag (json, yaml, plain)
2. Include more metadata (git status, diff stats)
3. Add session state information

### Phase 3: Future AI Support
Structure supports other AIs by changing `ai_command`:
```json
{
  "terminal_config": {
    "ai_command": "aider",
    "ai_args": ["--yes", "--model", "gpt-4"],
    "ai_backend": "aider"
  }
}
```

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_terminal_mode_start_output() {
    let args = StartArgs {
        terminal: true,
        name: Some("test-session".to_string()),
        prompt: Some("test prompt".to_string()),
        ..Default::default()
    };
    
    let result = execute_terminal_start(args, &test_config());
    assert!(result.is_ok());
    // Verify JSON structure
}

#[test]
fn test_terminal_mode_blocks_docker() {
    let args = StartArgs {
        terminal: true,
        container: true,
        ..Default::default()
    };
    
    let result = execute_terminal_start(args, &test_config());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not supported"));
}
```

### Integration Tests
1. Create session with `--terminal`, verify JSON output
2. Resume session with `--terminal`, verify session ID detected
3. Test error cases return proper JSON errors
4. Verify no side effects (no IDE launch, no task files)

## Documentation Updates

### CLI Help Text
```
para start --help
  ...
  --terminal    Output terminal configuration as JSON instead of launching IDE.
                Useful for integrating with terminal-based tools.
                Note: Sandboxing and Docker not supported in terminal mode.
```

### README Addition
```markdown
## Terminal Mode

Para can output session configuration as JSON for integration with other tools:

\```bash
# Start a new session and get terminal config
para start my-feature --terminal --dangerously-skip-permissions

# Resume an existing session
para resume my-feature --terminal
\```

This mode is useful for:
- Terminal-based AI interfaces (para-ui)
- Custom automation scripts  
- CI/CD pipelines

Current limitations:
- No sandbox support
- No Docker container support
- Returns configuration only, doesn't launch anything
```

## Success Criteria
1. ✅ `para start --terminal` returns valid JSON with all needed session info
2. ✅ `para resume --terminal` detects Claude session ID and includes in output
3. ✅ Dangerous permissions flag properly passed through
4. ✅ Proper error messages as JSON for invalid inputs
5. ✅ No side effects (no IDE launch, no temp files beyond worktree)
6. ✅ Existing para behavior unchanged when --terminal not used
7. ✅ Clear error when terminal mode used with unsupported features

## Future Considerations
1. **Sandbox Support**: Could return sandbox wrapper commands in JSON
2. **Docker Support**: Could return container connection info
3. **Progress Streaming**: Could support `--stream` for real-time updates
4. **Session Metadata**: Could include git status, recent commits, etc.
5. **AI Abstraction**: Config could specify preferred AI backend per session