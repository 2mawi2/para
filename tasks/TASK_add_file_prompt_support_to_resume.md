# Add file and prompt support to para resume command

## Goal
Enhance the `para resume` command to support passing additional context via files or inline prompts, similar to how `para dispatch` works. This will allow users to provide updated instructions or context when resuming existing sessions.

## Requirements

### 1. Update CLI Parser
Modify `ResumeArgs` in `src/cli/parser.rs` to add:
```rust
#[derive(Args, Debug)]
pub struct ResumeArgs {
    /// Session ID to resume (optional, shows list if not provided)
    pub session: Option<String>,
    
    /// Additional prompt or instructions for the resumed session
    #[arg(long, short)]
    pub prompt: Option<String>,
    
    /// Read additional instructions from specified file
    #[arg(long, short)]
    pub file: Option<PathBuf>,
}
```

### 2. Update Resume Command Implementation
In `src/cli/commands/resume.rs`:
- Add logic to handle the new prompt and file arguments
- If file is provided, read its contents
- If prompt is provided, use it directly
- Pass the additional context to the IDE launch command (if supported)
- Store the additional context in session metadata for reference

### 3. MCP Server Updates
Update `mcp-server-ts/src/para-mcp-server.ts`:
- Add `prompt` and `file` parameters to the `para_resume` tool schema
- Pass these parameters to the resume command when provided
- Update tool description to mention the new capabilities

### 4. Implementation Details
- File paths should be resolved relative to current directory
- Support both absolute and relative paths
- Validate file exists and is readable
- Handle edge cases:
  - Both prompt and file provided (error)
  - File doesn't exist (error with helpful message)
  - Empty file (warning but continue)
  - Large files (set reasonable size limit, e.g., 1MB)

### 5. IDE Integration Considerations
- The additional context could be:
  - Saved to a `.para/sessions/{session}/resume_context.md` file
  - Passed as environment variable `PARA_RESUME_CONTEXT`
  - Displayed in the IDE if it supports it
  - At minimum, saved for reference even if IDE doesn't use it

### 6. Usage Examples
```bash
# Resume with inline prompt
para resume my-session --prompt "Continue implementing the auth system"

# Resume with file containing instructions
para resume my-session --file updates/new_requirements.md

# Resume interactively (existing behavior)
para resume
```

### 7. Testing Requirements
- Unit tests for argument parsing
- Integration tests for file reading
- Test error cases (missing file, both args, etc.)
- Test MCP tool with new parameters
- Ensure backwards compatibility (resume without args still works)

### 8. Documentation
- Update help text for resume command
- Add examples to README
- Update MCP documentation
- Add to command reference

## Expected Behavior
- `para resume session-name` - Works as before
- `para resume session-name --prompt "text"` - Resumes with additional context
- `para resume session-name --file context.md` - Resumes with file contents as context
- The additional context is preserved and accessible in the resumed session

When done: para finish "Add file and prompt support to resume command"